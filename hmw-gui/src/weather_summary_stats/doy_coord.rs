use std::ops::Range;

use chrono::{Datelike, Duration, NaiveDate};
use plotters::coord::ranged1d::{
    DiscreteRanged, KeyPointHint, NoDefaultFormatting, Ranged, ValueFormatter,
};

/// A date coordinate that emits month-start key points for day-of-year charts.
#[derive(Clone)]
pub(crate) struct DayOfYearly(pub(crate) Range<NaiveDate>);

impl From<Range<NaiveDate>> for DayOfYearly {
    fn from(range: Range<NaiveDate>) -> Self {
        Self(range)
    }
}

impl DayOfYearly {
    fn first_month_start(&self) -> NaiveDate {
        let start = self.0.start;

        match start.day() == 1 {
            true => start,
            false => next_month(start.year(), start.month()),
        }
    }

    fn month_key_points(&self) -> Vec<NaiveDate> {
        let mut date = self.first_month_start();
        let mut key_points = vec![];

        while date < self.0.end {
            if date >= self.0.start {
                key_points.push(date);
            }

            date = next_month(date.year(), date.month());
        }

        key_points
    }
}

impl ValueFormatter<NaiveDate> for DayOfYearly {
    fn format(value: &NaiveDate) -> String {
        format!("{} {}", month_abbreviation(value.month()), value.day())
    }
}

impl Ranged for DayOfYearly {
    type FormatOption = NoDefaultFormatting;
    type ValueType = NaiveDate;

    fn range(&self) -> Range<NaiveDate> {
        self.0.start..self.0.end
    }

    fn map(&self, value: &Self::ValueType, limit: (i32, i32)) -> i32 {
        let total_days = (self.0.end - self.0.start).num_days() as f64;
        let value_days = (*value - self.0.start).num_days() as f64;

        match total_days > 0.0 {
            true => (f64::from(limit.1 - limit.0) * value_days / total_days) as i32 + limit.0,
            false => limit.0,
        }
    }

    fn key_points<Hint: KeyPointHint>(&self, hint: Hint) -> Vec<Self::ValueType> {
        let max_num_points = hint.max_num_points();

        match max_num_points {
            0 => vec![],
            _ => {
                let month_key_points = self.month_key_points();
                let stride = month_key_points.len().div_ceil(max_num_points).max(1);

                month_key_points.into_iter().step_by(stride).collect()
            }
        }
    }
}

impl DiscreteRanged for DayOfYearly {
    fn size(&self) -> usize {
        (self.0.end - self.0.start).num_days().max(0) as usize
    }

    fn index_of(&self, value: &NaiveDate) -> Option<usize> {
        let index = (*value - self.0.start).num_days();

        match *value >= self.0.start && *value < self.0.end && index >= 0 {
            true => Some(index as usize),
            false => None,
        }
    }

    fn from_index(&self, index: usize) -> Option<NaiveDate> {
        let value = self.0.start + Duration::days(index as i64);

        match value < self.0.end {
            true => Some(value),
            false => None,
        }
    }
}

fn next_month(year: i32, month: u32) -> NaiveDate {
    match month == 12 {
        true => NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap(),
        false => NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap(),
    }
}

fn month_abbreviation(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    }
}
