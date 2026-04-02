use map_3d::{ecef2geodetic, geodetic2ecef};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct ECEFPoint(pub [f64; 3]);

impl From<geo::Point> for ECEFPoint {
    fn from(value: geo::Point) -> Self {
        let (x, y) = value.to_radians().x_y();
        let (x, y, z) = geodetic2ecef(y, x, 0., Default::default());
        Self([x, y, z])
    }
}

impl From<ECEFPoint> for geo::Point {
    fn from(value: ECEFPoint) -> Self {
        let (y, x, alt) = ecef2geodetic(value.0[0], value.0[1], value.0[2], Default::default());
        debug_assert!(alt.abs() < 10., "Altitude is low");
        geo::Point::new(map_3d::rad2deg(x), map_3d::rad2deg(y))
    }
}
