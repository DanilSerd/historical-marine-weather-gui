use std::borrow::Borrow;

use chrono::{NaiveDate, NaiveTime};

use crate::types::{DateTime, IMMARecord};

pub trait DateTimeExt {
    fn date(&self) -> Option<NaiveDate>;
    fn time(&self) -> Option<NaiveTime>;
}

impl DateTimeExt for DateTime {
    fn date(&self) -> Option<NaiveDate> {
        self.date
    }

    fn time(&self) -> Option<NaiveTime> {
        Some(self.time.as_ref()?.time.0)
    }
}

impl<T> DateTimeExt for Option<T>
where
    T: Borrow<DateTime>,
{
    fn date(&self) -> Option<NaiveDate> {
        self.as_ref().map(|t| t.borrow()).and_then(|t| t.date())
    }

    fn time(&self) -> Option<NaiveTime> {
        self.as_ref().map(|t| t.borrow()).and_then(|t| t.time())
    }
}

impl DateTimeExt for IMMARecord {
    fn date(&self) -> Option<NaiveDate> {
        self.date_time.date()
    }

    fn time(&self) -> Option<NaiveTime> {
        self.date_time.time()
    }
}
