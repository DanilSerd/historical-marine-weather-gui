use chrono::{NaiveDate, NaiveTime};
use hmw_geo::LatticedPoint;
use hmw_parquet::datafusion::prelude::{Expr, col, get_field};
use imma_parser::{traits::WindExt, types::WindDir};

use crate::{GetDate, GetTime, GetYear};

use super::Project;

pub struct WindObservation {
    /// Lattice point of the observation.
    pub latticed_point: Option<LatticedPoint>,
    /// Year
    pub year: Option<u16>,
    /// Month
    pub month: Option<u8>,
    /// Day
    pub day: Option<u8>,
    /// Time of the observation.
    pub time: Option<NaiveTime>,
    /// Wind direction.
    pub wind_direction: Option<WindDir>,
    /// Wind speed.
    pub wind_speed: Option<f32>,
}

impl Project for WindObservation {
    fn columns() -> Vec<Expr> {
        vec![
            col("latticed_point"),
            col("year"),
            col("month"),
            col("day"),
            col("time"),
            col("wind"),
        ]
    }

    fn project(data: super::MarineWeatherObservation) -> Self {
        Self {
            latticed_point: data.latticed_point,
            year: data.year,
            month: data.month,
            day: data.day,
            time: data.time.map(|t| t.0),
            wind_speed: data.wind.wind_speed(),
            wind_direction: data.wind.wind_direction(),
        }
    }

    fn filter() -> Expr {
        get_field(col("wind"), "direction")
            .is_not_null()
            .or(get_field(col("wind"), "speed").is_not_null())
    }
}

impl GetYear for WindObservation {
    fn get(&self) -> Option<u16> {
        self.year
    }
}

impl GetDate for WindObservation {
    fn get(&self) -> Option<NaiveDate> {
        self.year
            .zip(self.month)
            .zip(self.day)
            .and_then(|((y, m), d)| NaiveDate::from_ymd_opt(y as i32, m as u32, d as u32))
    }
}

impl GetTime for WindObservation {
    fn get(&self) -> Option<NaiveTime> {
        self.time
    }
}
