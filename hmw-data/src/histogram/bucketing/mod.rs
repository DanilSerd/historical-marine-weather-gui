use ndhistogram::axis::Axis;
use thiserror::Error;

use super::CardinalOrdinalDirection;

mod beaufort;
mod douglas_waves;

pub use beaufort::{BeaufortScaleBucket, BeaufortScaleBucketer};
pub use douglas_waves::{DouglasWavesBucket, DouglasWavesBucketer};

#[derive(Debug, Clone, Copy, Hash, Error, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirectionalBucketingError {
    /// Direction is unknown, intensity is known.
    #[error("Unknown direction")]
    UnknownDirection,
    /// Intensity is unknown, direction is known.
    #[error("Unknown intensity")]
    UnknownIntensity,
    /// Both direction and intensity is unknown.
    #[error("Unknown direction and intensity")]
    UnknownDirectionIntensity,
    /// Direction and Intensity are inconsistent with each other. e.g. Calm direction, but high
    /// intensity.
    #[error("Inconsistent direction/intensity")]
    Inconsistent,
}

/// Trait for directional bucketing.
/// Implementing this trait allows for custom bucketing of directional data in the histogram.
pub trait DirectionalBucketing: Axis<Coordinate = f32> + Default {
    type Observation;

    /// Process the observation and return the direction and intensity in that direction.
    fn process(
        obs: &Self::Observation,
    ) -> Result<(CardinalOrdinalDirection, Self::Coordinate), DirectionalBucketingError>;
}
