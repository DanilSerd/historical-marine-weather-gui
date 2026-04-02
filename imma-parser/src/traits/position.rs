use std::borrow::Borrow;

use crate::types::{IMMARecord, Position};

pub trait PositionExt {
    fn position(&self) -> Option<(f64, f64)>;

    fn position_as_point(&self) -> Option<geo::Point<f64>> {
        self.position().map(|(x, y)| geo::Point::new(x, y))
    }

    fn position_geo_hash(&self, len: usize) -> Result<Option<String>, geohash::GeohashError> {
        self.position_as_point()
            .map(|p| geohash::encode(p.into(), len))
            .transpose()
    }
}

impl PositionExt for Position {
    fn position(&self) -> Option<(f64, f64)> {
        Some((self.lo as f64, self.la as f64))
    }
}

impl<T> PositionExt for Option<T>
where
    T: Borrow<Position>,
{
    fn position(&self) -> Option<(f64, f64)> {
        self.as_ref().and_then(|p| p.borrow().position())
    }
}

impl PositionExt for IMMARecord {
    fn position(&self) -> Option<(f64, f64)> {
        self.position.position()
    }
}
