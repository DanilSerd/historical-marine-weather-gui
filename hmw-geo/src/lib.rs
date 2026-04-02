mod geohash_lattice;
mod utils;

use std::path::Path;

pub use geo;
use geo::MultiPolygon;
pub use geohash;
pub use geohash_lattice::*;
pub use ordered_float;
use thiserror::Error;
pub use utils::*;

pub const WGS84_FLATTENING: f64 = 1.0 / 298.257223563;
pub const WGS84_MAJOR_AXIS: f64 = 6378137.0;

const BASE32_CODES: [u8; 32] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'b', b'c', b'd', b'e', b'f', b'g',
    b'h', b'j', b'k', b'm', b'n', b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z',
];

#[derive(Error, Debug)]
pub enum Error {
    #[error("SHP file error: {0}")]
    ShpFile(#[from] shapefile::Error),
}

pub fn lattice_with_shp_file_mask(
    mask_file: impl AsRef<Path>,
    progress: tokio::sync::mpsc::UnboundedSender<LatticeBuildProgress>,
) -> Result<Lattice, Error> {
    let polygon = read_shp_file(mask_file)?;
    let lattice = Lattice::new(&polygon, progress);
    Ok(lattice)
}

fn read_shp_file(path: impl AsRef<Path>) -> Result<MultiPolygon, Error> {
    let shapes = shapefile::read(path)?;
    let mut polygon = MultiPolygon::new(vec![]);
    for (s, _) in shapes {
        let mut poly: MultiPolygon = match s {
            shapefile::Shape::Polygon(p) => p.try_into()?,
            t => {
                return Err(shapefile::Error::MismatchShapeType {
                    requested: shapefile::ShapeType::Polygon,
                    actual: t.shapetype(),
                }
                .into());
            }
        };
        polygon.0.append(&mut poly.0);
    }
    Ok(polygon)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use tinystr::TinyAsciiStr;

    use crate::{Lattice, LatticeEntry, LatticeStats};

    #[test]
    fn test_lattice_simple() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let mask = geo::Polygon::new(
            vec![
                geo::Point::new(0., 0.),
                geo::Point::new(11.25, 0.),
                geo::Point::new(11.25, 7.03125),
                geo::Point::new(0., 7.03125),
            ]
            .into(),
            vec![],
        );
        let lattice = Lattice::new(&mask, tx);
        let stats = lattice.stats();

        assert_eq!(
            stats,
            LatticeStats {
                num_of_coarse_entries: 1,
                num_of_fine_entries: 8
            }
        );

        assert_eq!(lattice.entries.len(), 9);

        assert_eq!(
            lattice
                .entries
                .iter()
                .filter(|(e, _)| **e
                    == LatticeEntry::Coarse(TinyAsciiStr::<2>::from_str("s0").unwrap()))
                .count(),
            1
        );
        assert_eq!(
            lattice
                .entries
                .iter()
                .filter(|(e, _)| matches!(e, LatticeEntry::Fine(_)))
                .filter(|(e, _)| {
                    let a: &str = e.as_ref();
                    &a[0..2] == "s1"
                })
                .count(),
            8
        );
    }
}
