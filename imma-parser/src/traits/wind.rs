use std::borrow::Borrow;

use crate::types::{IMMARecord, Wind, WindDir};

pub trait WindExt {
    fn wind(&self) -> Option<&Wind>;

    fn wind_speed(&self) -> Option<f32> {
        Some(self.wind().as_ref()?.speed.as_ref()?.speed)
    }

    fn wind_direction(&self) -> Option<WindDir> {
        Some(self.wind().as_ref()?.direction.as_ref()?.direction)
    }
}

impl WindExt for Wind {
    fn wind(&self) -> Option<&Wind> {
        Some(self)
    }
}

impl<T> WindExt for Option<T>
where
    T: Borrow<Wind>,
{
    fn wind(&self) -> Option<&Wind> {
        self.as_ref().map(|w| w.borrow())
    }
}

impl WindExt for IMMARecord {
    fn wind(&self) -> Option<&Wind> {
        self.wind.wind()
    }
}
