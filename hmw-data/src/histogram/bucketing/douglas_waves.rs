use std::fmt::Display;

use imma_parser::types::WavesDirection;
use ndhistogram::axis::{Axis, Variable};
use strum::{EnumIter, FromRepr};

use crate::WavesObservation;
use crate::histogram::bucketing::DirectionalBucketingError;

use super::CardinalOrdinalDirection;

use super::DirectionalBucketing;

const DOUGLAS_WAVES_EDGES: [f32; 7] = [0.5, 1.25, 2.5, 4., 6., 9., 14.];

#[derive(Clone, Debug)]
pub struct DouglasWavesBucketer {
    inner_axis: Variable<f32>,
}

impl DouglasWavesBucketer {
    fn new() -> Self {
        Self {
            inner_axis: Variable::new(DOUGLAS_WAVES_EDGES).expect("correct douglas waves"),
        }
    }
}

impl Default for DouglasWavesBucketer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(FromRepr, EnumIter, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum DouglasWavesBucket {
    Calm = 0,
    Smooth,
    Slight,
    Moderate,
    Rough,
    VeryRough,
    High,
    VeryHigh,
    Phenomenal,
}

impl Axis for DouglasWavesBucketer {
    type Coordinate = <Variable<f32> as Axis>::Coordinate;

    type BinInterval = DouglasWavesBucket;

    fn index(&self, coordinate: &Self::Coordinate) -> Option<usize> {
        self.inner_axis.index(coordinate)
    }

    fn num_bins(&self) -> usize {
        self.inner_axis.num_bins()
    }

    fn bin(&self, index: usize) -> Option<Self::BinInterval> {
        Some(DouglasWavesBucket::from_repr(index as u8).expect("correct bucket index"))
    }
}

impl DirectionalBucketing for DouglasWavesBucketer {
    type Observation = WavesObservation;

    fn process(
        obs: &Self::Observation,
    ) -> Result<(CardinalOrdinalDirection, f32), DirectionalBucketingError> {
        match (obs.wave_height, obs.wave_direction) {
            (Some(h), _) if h < DOUGLAS_WAVES_EDGES[0] => {
                Ok((CardinalOrdinalDirection::Indeterminate, h))
            }
            (Some(h), Some(d @ WavesDirection::Direction(_))) => {
                let direction = CardinalOrdinalDirection::from(d);
                Ok((direction, h))
            }
            (
                None,
                None | Some(WavesDirection::IndeterminateLow | WavesDirection::IndeterminateHigh),
            ) => Err(DirectionalBucketingError::UnknownDirectionIntensity),
            (None, Some(WavesDirection::Direction(_))) => {
                Err(DirectionalBucketingError::UnknownIntensity)
            }
            (
                Some(_),
                None | Some(WavesDirection::IndeterminateLow | WavesDirection::IndeterminateHigh),
            ) => Err(DirectionalBucketingError::UnknownDirection),
        }
    }
}

impl Display for DouglasWavesBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DouglasWavesBucket::Calm => "Calm",
            DouglasWavesBucket::Smooth => "Smooth",
            DouglasWavesBucket::Slight => "Slight",
            DouglasWavesBucket::Moderate => "Moderate",
            DouglasWavesBucket::Rough => "Rough",
            DouglasWavesBucket::VeryRough => "Very Rough",
            DouglasWavesBucket::High => "High",
            DouglasWavesBucket::VeryHigh => "Very High",
            DouglasWavesBucket::Phenomenal => "Phenomenal",
        };
        write!(f, "{}", s)
    }
}
