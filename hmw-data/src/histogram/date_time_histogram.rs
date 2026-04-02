use std::ops::RangeInclusive;

use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use ndhistogram::{
    AxesTuple, Histogram, VecHistogram, axis::UniformNoFlow, ndhistogram, value::Sum,
};

use crate::histogram::stats::DateTimeHistogramCounters;

const FIRST_OF_JAN_2000: NaiveDate = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
const MIDNIGHT: NaiveTime = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
const FIRST_YEAR_IN_HISTOGRAM: i32 = 1660;

/// Histogram of observation counts by year, day-of-year, and hour-of-day.
#[derive(Debug, Clone)]
pub struct DateTimeHistogram {
    /// Year histogram.
    years: VecHistogram<AxesTuple<(UniformNoFlow<i32>,)>, Sum<usize>>,
    /// Day-of-year histogram on a 366 day year.
    doy: VecHistogram<AxesTuple<(UniformNoFlow<u32>,)>, Sum<usize>>,
    /// Hour-of-day histogram.
    hod: VecHistogram<AxesTuple<(UniformNoFlow<u32>,)>, Sum<usize>>,
    /// Counters for the construction.
    pub counters: DateTimeHistogramCounters,
}

/// Retrieve a date from an observation.
pub trait GetYear {
    /// Retrieve date.
    fn get(&self) -> Option<u16>;
}

/// Retrieve a date from an observation.
pub trait GetDate {
    /// Retrieve date.
    fn get(&self) -> Option<NaiveDate>;
}

/// Retrieve a time from an observation.
pub trait GetTime {
    /// Retrieve time.
    fn get(&self) -> Option<NaiveTime>;
}

/// One year bin from the date/time histogram.
pub struct YearHistogramBucket {
    /// Year.
    pub year: i32,
    /// Count of observations.
    pub count: usize,
}

/// One day-of-year bin from the date/time histogram.
pub struct DayOfYearHistogramBucket {
    /// Day of year. Zero based. Always treated as leap-year indexed.
    pub day: u32,
    /// Count of observations.
    pub count: usize,
}

/// One hour-of-day bin from the date/time histogram.
pub struct HourOfDayHistogramBucket {
    /// Hour of day.
    pub hour: u32,
    /// Count of observations.
    pub count: usize,
}

impl DateTimeHistogram {
    /// Create an empty date/time histogram.
    pub fn new() -> Self {
        let year_bin_count = Utc::now().date_naive().year() + 2 - FIRST_YEAR_IN_HISTOGRAM;
        let years = ndhistogram!(
            UniformNoFlow::with_step_size(year_bin_count as usize, FIRST_YEAR_IN_HISTOGRAM, 1)
                .expect("year axis is correct");
            Sum<usize>
        );
        let doy = ndhistogram!(
            UniformNoFlow::with_step_size(366, 0u32, 1).expect("doy axis is correct");
            Sum<usize>
        );
        let hod = ndhistogram!(
            UniformNoFlow::with_step_size(24, 0u32, 1).expect("hod axis is correct");
            Sum<usize>
        );

        Self {
            years,
            doy,
            hod,
            counters: DateTimeHistogramCounters::default(),
        }
    }

    /// Add one observation to the histogram.
    pub fn fill<T: GetDate + GetTime + GetYear>(&mut self, obs: &T) {
        match GetYear::get(obs) {
            Some(y) => {
                let year = y as i32;
                debug_assert!(year >= FIRST_YEAR_IN_HISTOGRAM);

                self.years.fill(&year);
            }
            None => {
                self.counters.missing_year += 1;
            }
        }
        match GetDate::get(obs) {
            Some(date) => {
                let day = (date.with_year(2000).expect("can set to 2000") - FIRST_OF_JAN_2000)
                    .num_days() as u32;
                self.doy.fill(&day);
            }
            None => {
                self.counters.missing_date += 1;
            }
        }

        match GetTime::get(obs) {
            Some(time) => {
                let hour = (time - MIDNIGHT).num_hours() as u32;
                self.hod.fill(&hour);
            }
            None => {
                self.counters.missing_time += 1;
            }
        }
    }

    /// Iterate year bins inside the requested range.
    pub fn iter_year<'s>(
        &'s self,
        range: RangeInclusive<i32>,
    ) -> impl Iterator<Item = YearHistogramBucket> + use<'s> {
        range.into_iter().map(|year| YearHistogramBucket {
            year,
            count: self.years.value(&year).map(|v| v.sum()).unwrap_or_default(),
        })
    }

    /// Iterate all day-of-year bins.
    pub fn iter_doy<'s>(&'s self) -> impl Iterator<Item = DayOfYearHistogramBucket> + use<'s> {
        (0u32..366).map(|day_of_year| DayOfYearHistogramBucket {
            day: day_of_year,
            count: self.doy.value(&day_of_year).expect("day bin exists").sum(),
        })
    }

    /// Iterate all hour-of-day bins.
    pub fn iter_hod<'s>(&'s self) -> impl Iterator<Item = HourOfDayHistogramBucket> + use<'s> {
        (0u32..24).map(|hour| HourOfDayHistogramBucket {
            hour,
            count: self.hod.value(&hour).expect("hour bin exists").sum(),
        })
    }

    /// Return the largest year-bin count in the requested range.
    pub fn max_year_count(&self, range: RangeInclusive<i32>) -> usize {
        self.iter_year(range)
            .map(|bucket| bucket.count)
            .max()
            .unwrap_or(0)
            .max(1)
    }

    /// Return the largest day-of-year-bin count.
    pub fn max_doy_count(&self) -> usize {
        self.doy
            .iter()
            .map(|bucket| bucket.value.sum())
            .max()
            .unwrap_or(0)
            .max(1)
    }

    /// Return the largest hour-of-day-bin count.
    pub fn max_hod_count(&self) -> usize {
        self.hod
            .iter()
            .map(|bucket| bucket.value.get())
            .max()
            .unwrap_or(0)
            .max(1)
    }
}

impl Default for DateTimeHistogram {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use crate::GetYear;

    use super::{DateTimeHistogram, GetDate, GetTime};
    use chrono::{Datelike, NaiveDate, NaiveTime};

    struct TestObservation {
        date: Option<NaiveDate>,
        time: Option<NaiveTime>,
    }

    impl GetYear for TestObservation {
        fn get(&self) -> Option<u16> {
            self.date.map(|d| d.year() as u16)
        }
    }

    impl GetDate for TestObservation {
        fn get(&self) -> Option<NaiveDate> {
            self.date
        }
    }

    impl GetTime for TestObservation {
        fn get(&self) -> Option<NaiveTime> {
            self.time
        }
    }

    #[test]
    fn test_date_time_histogram_counts_and_missing_values() {
        let observations = [
            TestObservation {
                date: NaiveDate::from_ymd_opt(1990, 1, 1),
                time: NaiveTime::from_hms_opt(0, 15, 0),
            },
            TestObservation {
                date: NaiveDate::from_ymd_opt(1990, 1, 1),
                time: NaiveTime::from_hms_opt(0, 45, 0),
            },
            TestObservation {
                date: NaiveDate::from_ymd_opt(1990, 1, 2),
                time: NaiveTime::from_hms_opt(23, 0, 0),
            },
            TestObservation {
                date: NaiveDate::from_ymd_opt(1992, 2, 29),
                time: NaiveTime::from_hms_opt(12, 0, 0),
            },
            TestObservation {
                date: NaiveDate::from_ymd_opt(1991, 3, 1),
                time: NaiveTime::from_hms_opt(12, 30, 0),
            },
            TestObservation {
                date: None,
                time: NaiveTime::from_hms_opt(5, 0, 0),
            },
            TestObservation {
                date: NaiveDate::from_ymd_opt(1990, 1, 3),
                time: None,
            },
        ];
        let mut histogram = DateTimeHistogram::new();

        observations
            .iter()
            .for_each(|observation| histogram.fill(observation));

        let year_counts = histogram
            .iter_year(1990..=1992)
            .filter(|count| count.count > 0)
            .map(|count| (count.year, count.count))
            .collect::<Vec<_>>();
        let doy_counts = histogram
            .iter_doy()
            .filter(|count| count.count > 0)
            .map(|count| (count.day, count.count))
            .collect::<Vec<_>>();
        let hod_counts = histogram
            .iter_hod()
            .filter(|count| count.count > 0)
            .map(|count| (count.hour, count.count))
            .collect::<Vec<_>>();

        assert_eq!(histogram.counters.missing_year, 1);
        assert_eq!(histogram.counters.missing_date, 1);
        assert_eq!(histogram.counters.missing_time, 1);
        assert_eq!(year_counts, vec![(1990, 4), (1991, 1), (1992, 1)]);
        assert_eq!(doy_counts, vec![(0, 2), (1, 1), (2, 1), (59, 1), (60, 1)]);
        assert_eq!(hod_counts, vec![(0, 2), (5, 1), (12, 2), (23, 1)]);
    }

    #[test]
    fn test_iter_year_includes_zero_count_years_inside_requested_range() {
        let observations = [
            TestObservation {
                date: NaiveDate::from_ymd_opt(1990, 1, 1),
                time: NaiveTime::from_hms_opt(0, 0, 0),
            },
            TestObservation {
                date: NaiveDate::from_ymd_opt(1992, 1, 1),
                time: NaiveTime::from_hms_opt(0, 0, 0),
            },
        ];
        let mut histogram = DateTimeHistogram::new();

        observations
            .iter()
            .for_each(|observation| histogram.fill(observation));

        assert_eq!(
            collect_year_counts(&histogram, 1990..=1993),
            vec![(1990, 1), (1991, 0), (1992, 1), (1993, 0)]
        );
    }

    fn collect_year_counts(
        histogram: &DateTimeHistogram,
        range: RangeInclusive<i32>,
    ) -> Vec<(i32, usize)> {
        histogram
            .iter_year(range)
            .map(|bucket| (bucket.year, bucket.count))
            .collect()
    }
}
