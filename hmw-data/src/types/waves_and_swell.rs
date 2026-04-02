use chrono::{NaiveDate, NaiveTime};
use hmw_geo::LatticedPoint;
use hmw_parquet::datafusion::prelude::{Expr, col, get_field};
use imma_parser::{
    traits::WavesExt,
    types::{NaiveTimeWrapper, Waves, WavesDirection},
};

use crate::{GetDate, GetTime, GetYear};

use super::Project;

pub struct WavesObservation {
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
    /// Wave height.
    pub wave_height: Option<f32>,
    /// Wave direction.
    pub wave_direction: Option<WavesDirection>,
    /// Wave length in deep water.
    pub wave_length_in_deep_water: Option<f32>,
}

impl WavesObservation {
    pub fn project_from_components(
        latticed_point: Option<LatticedPoint>,
        year: Option<u16>,
        month: Option<u8>,
        day: Option<u8>,
        time: Option<NaiveTimeWrapper>,
        waves: Option<Waves>,
    ) -> Self {
        Self {
            latticed_point,
            year,
            month,
            day,
            time: time.map(|t| t.0),
            wave_height: waves.wave_height(),
            wave_direction: waves.wave_direction(),
            wave_length_in_deep_water: waves.wave_length_in_deep_water(),
        }
    }

    pub fn filter(col: Expr) -> Expr {
        get_field(col.clone(), "direction")
            .is_not_null()
            .or(get_field(col.clone(), "height").is_not_null())
            .or(get_field(col.clone(), "period").is_not_null())
    }
}

impl Project for WavesObservation {
    fn columns() -> Vec<Expr> {
        vec![
            col("latticed_point"),
            col("year"),
            col("month"),
            col("day"),
            col("time"),
            col("waves"),
        ]
    }

    fn project(data: super::MarineWeatherObservation) -> Self {
        Self::project_from_components(
            data.latticed_point,
            data.year,
            data.month,
            data.day,
            data.time,
            data.waves,
        )
    }

    fn filter() -> Expr {
        Self::filter(col("waves"))
    }
}

impl GetYear for WavesObservation {
    fn get(&self) -> Option<u16> {
        self.year
    }
}

impl GetDate for WavesObservation {
    fn get(&self) -> Option<NaiveDate> {
        self.year
            .zip(self.month)
            .zip(self.day)
            .and_then(|((y, m), d)| NaiveDate::from_ymd_opt(y as i32, m as u32, d as u32))
    }
}

impl GetTime for WavesObservation {
    fn get(&self) -> Option<NaiveTime> {
        self.time
    }
}

pub struct SwellObservation(pub WavesObservation);

impl Project for SwellObservation {
    fn columns() -> Vec<Expr> {
        let mut columns = WavesObservation::columns();
        let last_column = columns.last_mut().expect("last column exists");
        *last_column = col("swell");

        columns
    }

    fn project(data: super::MarineWeatherObservation) -> Self {
        Self(WavesObservation::project_from_components(
            data.latticed_point,
            data.year,
            data.month,
            data.day,
            data.time,
            data.swell,
        ))
    }
    fn filter() -> Expr {
        WavesObservation::filter(col("swell"))
    }
}

impl GetYear for SwellObservation {
    fn get(&self) -> Option<u16> {
        GetYear::get(&self.0)
    }
}

impl GetDate for SwellObservation {
    fn get(&self) -> Option<NaiveDate> {
        GetDate::get(&self.0)
    }
}

impl GetTime for SwellObservation {
    fn get(&self) -> Option<NaiveTime> {
        self.0.time
    }
}
