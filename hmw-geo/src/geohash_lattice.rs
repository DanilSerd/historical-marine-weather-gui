use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display},
    fs::File,
    io::{BufReader, BufWriter},
    ops::Index,
    path::Path,
    str::FromStr,
};

use either::Either;
use geo::{BooleanOps, GeodesicArea, Point, Polygon, Relate, Triangle, TriangulateEarcut};
use rayon::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tinystr::TinyAsciiStr;

use super::BASE32_CODES;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Lattice {
    /// Entries of the lattice.
    /// The key is the entry and the value is the index of the entry in the lattice. As sorted by
    /// geohash.
    pub(crate) entries: HashMap<LatticeEntry, usize>,
}

/// Lattice entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LatticeEntry {
    /// Geohash with length 3 precision.
    Fine(TinyAsciiStr<3>),
    /// Geohash with length 2 precision.
    Coarse(TinyAsciiStr<2>),
}

impl TryFrom<(Point, u8)> for LatticeEntry {
    type Error = &'static str;

    fn try_from(value: (Point, u8)) -> Result<Self, Self::Error> {
        let hash = geohash::encode(value.0.into(), value.1 as usize)
            .map_err(|_| "Failed to encode point to geohash")?;
        if value.1 >= 3 {
            Ok(LatticeEntry::Fine(
                TinyAsciiStr::<3>::from_str(&hash[0..3]).unwrap(),
            ))
        } else if value.1 == 2 {
            Ok(LatticeEntry::Coarse(
                TinyAsciiStr::<2>::from_str(&hash[0..2]).unwrap(),
            ))
        } else {
            Err("Invalid length for lattice entry geohash")
        }
    }
}

impl Serialize for LatticeEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

impl<'de> Deserialize<'de> for LatticeEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        if s.len() == 2 {
            Ok(LatticeEntry::Coarse(
                TinyAsciiStr::<2>::from_str(&s).unwrap(),
            ))
        } else if s.len() == 3 {
            Ok(LatticeEntry::Fine(TinyAsciiStr::<3>::from_str(&s).unwrap()))
        } else {
            Err(serde::de::Error::custom(
                "Invalid length for lattice entry geohash",
            ))
        }
    }
}

/// Point placed in the lattice. Just a wrapper around a geohash of length 8 precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LatticedPoint(pub(crate) TinyAsciiStr<8>);

impl Lattice {
    /// Create a new geohash lattice using a mask.
    /// The lattice will contain course and fine entries.
    /// If the mask doesn't contain the bbox of the geohash of precision 2, this bbox (or any sub-bbox) will not be in the lattice.
    /// If the bbox of precision 2 is fully contained in the mask it will be [`LatticeEntry::Coarse`].
    /// If the bbox of precision 3 is fully contained or intersects the mask it will be [`LatticeEntry::Fine`].
    pub fn new<M>(
        mask: &M,
        progress: tokio::sync::mpsc::UnboundedSender<LatticeBuildProgress>,
    ) -> Self
    where
        M: Relate<f64> + BooleanOps<Scalar = f64> + GeodesicArea<f64> + Sync,
    {
        let _ = progress.send(LatticeBuildProgress::Start(
            BASE32_CODES.len() * BASE32_CODES.len(),
        ));
        let iter = BASE32_CODES
            .par_iter()
            .flat_map(|c1| {
                BASE32_CODES
                    .par_iter()
                    .map(|c2| TinyAsciiStr::<2>::try_from_raw([*c1, *c2]).unwrap())
            })
            .filter_map(|h| {
                let corse_polygon = geohash::decode_bbox(&h).unwrap().to_polygon();
                let relate_matrix = mask.relate(&corse_polygon);
                if relate_matrix.is_intersects() && !relate_matrix.is_touches() {
                    return Some((h, relate_matrix));
                }
                let _ = progress.send(LatticeBuildProgress::Checked);
                None
            })
            .flat_map_iter(|(h, relate_matrix)| {
                let entries = if relate_matrix.is_contains() {
                    let coare_iter = std::iter::once(LatticeEntry::Coarse(h));
                    Either::Left(coare_iter)
                } else {
                    let fine_iter = BASE32_CODES
                        .iter()
                        .map(move |c| h.concat(TinyAsciiStr::<1>::try_from_raw([*c]).unwrap()))
                        .filter_map(|c| {
                            let fine_polygon = geohash::decode_bbox(&c).unwrap().to_polygon();
                            let relate_matrix = mask.relate(&fine_polygon);
                            (relate_matrix.is_intersects() && !relate_matrix.is_touches())
                                .then_some(LatticeEntry::Fine(c))
                        });
                    Either::Right(fine_iter)
                };

                let _ = progress.send(LatticeBuildProgress::Checked);
                entries
            });

        let mut entries_vec = iter.collect::<Vec<_>>();
        entries_vec.sort();

        Self {
            entries: entries_vec
                .into_iter()
                .enumerate()
                .map(|(i, e)| (e, i))
                .collect(),
        }
    }

    /// Returns the entry and index of the entry containing the point.
    pub fn containing(&self, point: Point) -> Option<(&LatticeEntry, &usize)> {
        let hash = geohash::encode(point.into(), 3).ok()?;
        let coarse_hash = TinyAsciiStr::<2>::from_str(&hash[0..2]).unwrap();
        let fine_hash = TinyAsciiStr::<3>::from_str(&hash[0..3]).unwrap();

        self.entries
            .get_key_value(&LatticeEntry::Coarse(coarse_hash))
            .or_else(|| self.entries.get_key_value(&LatticeEntry::Fine(fine_hash)))
    }

    pub fn iter_ordered(&self) -> impl IntoIterator<Item = (&LatticeEntry, &usize)> {
        let mut v: Vec<_> = self.entries.iter().collect();
        v.sort_by_key(|(_, i)| **i);
        v
    }

    pub fn lookup(&self, entry: &LatticeEntry) -> Option<&usize> {
        self.entries.get(entry)
    }

    pub fn stats(&self) -> LatticeStats {
        LatticeStats {
            num_of_coarse_entries: self
                .entries
                .iter()
                .filter(|(e, _)| matches!(e, LatticeEntry::Coarse(_)))
                .count(),
            num_of_fine_entries: self
                .entries
                .iter()
                .filter(|(e, _)| matches!(e, LatticeEntry::Fine(_)))
                .count(),
        }
    }
}

impl TryFrom<Point> for LatticedPoint {
    type Error = &'static str;

    fn try_from(value: Point) -> Result<Self, Self::Error> {
        let hash =
            geohash::encode(value.into(), 8).map_err(|_| "Failed to encode point to geohash")?;
        Ok(LatticedPoint(TinyAsciiStr::<8>::from_str(&hash).unwrap()))
    }
}

impl TryFrom<[u8; 8]> for LatticedPoint {
    type Error = &'static str;

    fn try_from(value: [u8; 8]) -> Result<Self, Self::Error> {
        if value.iter().any(|b| !BASE32_CODES.contains(b)) {
            return Err("Invalid character in lattice entry geohash");
        }
        Ok(LatticedPoint(
            TinyAsciiStr::<8>::try_from_utf8(&value).unwrap(),
        ))
    }
}

impl AsRef<[u8; 8]> for LatticedPoint {
    fn as_ref(&self) -> &[u8; 8] {
        self.0.all_bytes()
    }
}

impl AsRef<str> for LatticedPoint {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Index<usize> for LatticedPoint {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0.all_bytes()[index]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LatticeStats {
    pub num_of_coarse_entries: usize,
    pub num_of_fine_entries: usize,
}

impl Display for LatticeStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "course entries (length 2 hash): {}, fine entries (length 3 hash): {}",
            self.num_of_coarse_entries, self.num_of_fine_entries,
        )
    }
}

impl Lattice {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let u = serde_json::from_reader(reader)?;

        Ok(u)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let u = serde_json::from_slice(bytes)?;
        Ok(u)
    }

    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let file = File::options()
            .write(true)
            .truncate(true)
            .append(false)
            .create(true)
            .open(path)?;
        let writer = BufWriter::new(file);

        serde_json::to_writer(writer, self)?;

        Ok(())
    }
}

impl LatticeEntry {
    pub fn triangulate(&self) -> Box<[Triangle]> {
        match self.fine_entries() {
            Some(fine_iter) => fine_iter
                .flat_map(|e| e.bbox().earcut_triangles_iter())
                .collect(),
            None => self.bbox().earcut_triangles_iter().collect(),
        }
    }

    fn bbox(&self) -> Polygon {
        match self {
            LatticeEntry::Coarse(h) => geohash::decode_bbox(h).unwrap().to_polygon(),
            LatticeEntry::Fine(h) => geohash::decode_bbox(h).unwrap().to_polygon(),
        }
    }

    fn fine_entries(self) -> Option<impl Iterator<Item = LatticeEntry>> {
        match self {
            LatticeEntry::Fine(_) => None,
            LatticeEntry::Coarse(h) => Some(
                BASE32_CODES
                    .iter()
                    .map(move |c| h.concat(TinyAsciiStr::<1>::try_from_raw([*c]).unwrap()))
                    .map(LatticeEntry::Fine),
            ),
        }
    }

    pub fn geodesic_area_unsigned(&self) -> f64 {
        self.bbox().geodesic_area_unsigned()
    }
}

impl AsRef<str> for LatticeEntry {
    fn as_ref(&self) -> &str {
        match self {
            LatticeEntry::Coarse(h) => h.as_ref(),
            LatticeEntry::Fine(h) => h.as_ref(),
        }
    }
}

impl AsRef<[u8]> for LatticeEntry {
    fn as_ref(&self) -> &[u8] {
        match self {
            LatticeEntry::Coarse(h) => h.all_bytes(),
            LatticeEntry::Fine(h) => h.all_bytes(),
        }
    }
}

impl TryFrom<&str> for LatticeEntry {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.as_bytes().iter().any(|b| !BASE32_CODES.contains(b)) {
            return Err("Invalid character in lattice entry geohash");
        }
        match value.len() {
            2 => Ok(LatticeEntry::Coarse(
                TinyAsciiStr::<2>::from_str(value).unwrap(),
            )),
            3 => Ok(LatticeEntry::Fine(
                TinyAsciiStr::<3>::from_str(value).unwrap(),
            )),
            _ => Err("Invalid length for lattice entry geohash"),
        }
    }
}

impl arrow_convert::field::ArrowField for LatticedPoint {
    type Type = Self;

    fn data_type() -> arrow::datatypes::DataType {
        arrow::datatypes::DataType::Utf8
    }
}

impl arrow_convert::serialize::ArrowSerialize for LatticedPoint {
    type ArrayBuilderType = arrow::array::StringBuilder;

    fn new_array() -> Self::ArrayBuilderType {
        arrow::array::StringBuilder::new()
    }

    fn arrow_serialize(
        v: &<Self as arrow_convert::field::ArrowField>::Type,
        array: &mut Self::ArrayBuilderType,
    ) -> arrow::error::Result<()> {
        array.append_value(AsRef::<str>::as_ref(v));
        Ok(())
    }
}

impl arrow_convert::deserialize::ArrowDeserialize for LatticedPoint {
    type ArrayType = arrow::array::StringArray;

    fn arrow_deserialize(
        v: <Self::ArrayType as arrow_convert::deserialize::ArrowArrayIterable>::Item<'_>,
    ) -> Option<<Self as arrow_convert::field::ArrowField>::Type> {
        String::arrow_deserialize(v)
            .and_then(|s| TinyAsciiStr::<8>::from_str(&s).ok().map(LatticedPoint))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LatticeBuildProgress {
    Start(usize),
    Checked,
}
