use std::collections::HashMap;

use super::{bucketing::DirectionalBucketingError, date_time_histogram::DateTimeHistogram};

/// Counters for the date/time histogram.
#[derive(Debug, Default, Clone)]
pub struct DateTimeHistogramCounters {
    /// Number of observations missing year. Skipped by the year histogram.
    pub missing_year: usize,
    /// Number of observations that are missing date. Skipped by DOY histogram.
    pub missing_date: usize,
    /// Number of observations that are missing time. Skipped by the HOD histogram.
    pub missing_time: usize,
}

/// Counters for the main directional histogram.
#[derive(Debug, Clone, Default)]
pub struct HistogramCounters {
    /// Count of observations inserted into the histogram.
    pub inserted: usize,
    /// Count of observations skipped by the histogram as they didn't contain valid data.
    pub skipped: HashMap<DirectionalBucketingError, usize>,
}

impl HistogramCounters {
    pub fn add<T>(&mut self, obs: Result<T, DirectionalBucketingError>) -> Option<T> {
        match obs {
            Ok(t) => {
                self.inserted += 1;
                Some(t)
            }
            Err(e) => {
                *self.skipped.entry(e).or_default() += 1;
                None
            }
        }
    }
}

/// Borrowed histogram stats for UI display.
#[derive(Debug)]
pub struct HistogramStats<'a> {
    /// Year, day-of-year, and hour-of-day histogram.
    pub date_time: &'a DateTimeHistogram,
    /// Counters for this histogram.
    pub histogram_counters: &'a HistogramCounters,
}
