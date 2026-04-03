use std::fmt::Display;

use imma_parser::types::WindDir;
use ndhistogram::axis::{Axis, Variable};
use strum::{EnumIter, FromRepr};

use super::{CardinalOrdinalDirection, DirectionalBucketing, DirectionalBucketingError};

use crate::WindObservation;

const BEAUFORT_EDGES: [f32; 12] = [
    0.3, 1.6, 3.4, 5.5, 8.0, 10.8, 13.9, 17.2, 20.8, 24.5, 28.5, 32.7,
];

#[derive(Clone, Debug)]
pub struct BeaufortScaleBucketer {
    inner_axis: Variable<f32>,
}

impl BeaufortScaleBucketer {
    fn new() -> Self {
        Self {
            inner_axis: Variable::new(BEAUFORT_EDGES).expect("correct beaufort"),
        }
    }
}

impl Default for BeaufortScaleBucketer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(FromRepr, EnumIter, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum BeaufortScaleBucket {
    Calm = 0,
    LightAir,
    LightBreeze,
    GentleBreeze,
    ModerateBreeze,
    FreshBreeze,
    StrongBreeze,
    NearGale,
    FreshGale,
    StrongGale,
    Storm,
    ViolentStorm,
    Hurricane,
}

impl Axis for BeaufortScaleBucketer {
    type Coordinate = <Variable<f32> as Axis>::Coordinate;

    type BinInterval = BeaufortScaleBucket;

    fn index(&self, coordinate: &Self::Coordinate) -> Option<usize> {
        self.inner_axis.index(coordinate)
    }

    fn num_bins(&self) -> usize {
        self.inner_axis.num_bins()
    }

    fn bin(&self, index: usize) -> Option<Self::BinInterval> {
        Some(BeaufortScaleBucket::from_repr(index as u8).expect("correct bucket index"))
    }
}

impl DirectionalBucketing for BeaufortScaleBucketer {
    type Observation = WindObservation;

    fn process(
        obs: &Self::Observation,
    ) -> Result<(CardinalOrdinalDirection, f32), DirectionalBucketingError> {
        match (obs.wind_direction, obs.wind_speed) {
            (_, Some(s)) if s < BEAUFORT_EDGES[0] => {
                Ok((CardinalOrdinalDirection::Indeterminate, 0.))
            }
            (Some(WindDir::Calm), None) => Ok((CardinalOrdinalDirection::Indeterminate, 0.)),
            (Some(d @ WindDir::Direction(_)), Some(s)) => Ok((d.into(), s)),
            (None | Some(WindDir::Variable), None) => {
                Err(DirectionalBucketingError::UnknownDirectionIntensity)
            }
            (None | Some(WindDir::Variable), Some(_)) => {
                Err(DirectionalBucketingError::UnknownDirection)
            }
            (Some(WindDir::Direction(_)), None) => Err(DirectionalBucketingError::UnknownIntensity),
            (Some(WindDir::Calm), Some(_)) => Err(DirectionalBucketingError::Inconsistent),
        }
    }
}

impl Display for BeaufortScaleBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BeaufortScaleBucket::Calm => "Calm (<1 kt)",
            BeaufortScaleBucket::LightAir => "Light Air (1-3 kt)",
            BeaufortScaleBucket::LightBreeze => "Light Breeze (4-6 kt)",
            BeaufortScaleBucket::GentleBreeze => "Gentle Breeze (7-10 kt)",
            BeaufortScaleBucket::ModerateBreeze => "Moderate Breeze (11-16 kt)",
            BeaufortScaleBucket::FreshBreeze => "Fresh Breeze (17-21 kt)",
            BeaufortScaleBucket::StrongBreeze => "Strong Breeze (22-27 kt)",
            BeaufortScaleBucket::NearGale => "Near Gale (28-33 kt)",
            BeaufortScaleBucket::FreshGale => "Fresh Gale (34-40 kt)",
            BeaufortScaleBucket::StrongGale => "Strong Gale (41-47 kt)",
            BeaufortScaleBucket::Storm => "Storm (48-55 kt)",
            BeaufortScaleBucket::ViolentStorm => "Violent Storm (56-63 kt)",
            BeaufortScaleBucket::Hurricane => "Hurricane (>=64 kt)",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use ndhistogram::axis::Axis;

    use super::{BeaufortScaleBucket, BeaufortScaleBucketer};

    #[test]
    fn test_beaufort_scale_bucketer() {
        let beaufort_scale_bucketer = BeaufortScaleBucketer::default();
        let bins = beaufort_scale_bucketer.bins().collect::<Vec<_>>();
        assert_eq!(bins.len(), 13);
        assert_eq!(bins[0], BeaufortScaleBucket::Calm);
        assert_eq!(bins[1], BeaufortScaleBucket::LightAir);
        assert_eq!(bins[2], BeaufortScaleBucket::LightBreeze);
        assert_eq!(bins[3], BeaufortScaleBucket::GentleBreeze);
        assert_eq!(bins[4], BeaufortScaleBucket::ModerateBreeze);
        assert_eq!(bins[5], BeaufortScaleBucket::FreshBreeze);
        assert_eq!(bins[6], BeaufortScaleBucket::StrongBreeze);
        assert_eq!(bins[7], BeaufortScaleBucket::NearGale);
        assert_eq!(bins[8], BeaufortScaleBucket::FreshGale);
        assert_eq!(bins[9], BeaufortScaleBucket::StrongGale);
        assert_eq!(bins[10], BeaufortScaleBucket::Storm);
        assert_eq!(bins[11], BeaufortScaleBucket::ViolentStorm);
        assert_eq!(bins[12], BeaufortScaleBucket::Hurricane);
    }
}
