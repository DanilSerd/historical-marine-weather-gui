use std::borrow::Borrow;

use crate::types::{WavePeriod, Waves, WavesDirection};

pub trait WavesExt {
    fn waves(&self) -> Option<&Waves>;

    fn wave_height(&self) -> Option<f32> {
        self.waves()?.height
    }

    fn wave_period(&self) -> Option<WavePeriod> {
        self.waves()?.period
    }

    /// Wave length in deep water in meters.
    /// Accurate if depth is greater that 1/20th of the wave length.
    fn wave_length_in_deep_water(&self) -> Option<f32> {
        self.wave_period().and_then(|p| match p {
            WavePeriod::Period(p) => Some(1.560_776_8 * (p as f32).powi(2)),
            WavePeriod::Indeterminate => None,
        })
    }

    fn wave_direction(&self) -> Option<WavesDirection> {
        self.waves()?.direction
    }
}

impl WavesExt for Waves {
    fn waves(&self) -> Option<&Waves> {
        Some(self)
    }
}

impl<T> WavesExt for Option<T>
where
    T: Borrow<Waves>,
{
    fn waves(&self) -> Option<&Waves> {
        self.as_ref().map(|w| w.borrow())
    }
}
