use std::collections::HashSet;

use hmw_data::{BeaufortScaleBucketer, DirectionalIntensityHistogram, Epoch, HistogramStats};
use hmw_geo::LatticeEntry;
use serde::{Deserialize, Serialize};

use crate::loader::LoaderError;

#[derive(Debug, Clone)]
pub struct WeatherSummary {
    pub params: WeatherSummaryParams,
    pub data: WeatherSummaryData,
}

impl WeatherSummary {
    pub fn new(params: WeatherSummaryParams) -> Self {
        Self {
            params,
            data: WeatherSummaryData::None,
        }
    }

    pub fn invalidate_data(&mut self) {
        self.data = WeatherSummaryData::None;
    }
}

/// A header for a weather summary. Includes a unique identifier and a name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeatherSummaryHeader {
    pub id: WeatherSummaryId,
    pub name: String,
    pub summary_type: WeatherSummaryType,
}

const ALL_WEATHER_TYPES: [WeatherSummaryType; 1] = [WeatherSummaryType::Wind];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WeatherSummaryType {
    Wind,
}

impl WeatherSummaryType {
    pub fn all_types() -> &'static [WeatherSummaryType] {
        &ALL_WEATHER_TYPES[..]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WeatherSummaryId(uuid::Uuid);

impl std::fmt::Display for WeatherSummaryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl WeatherSummaryId {
    pub fn random() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl WeatherSummaryHeader {
    pub fn new(id: WeatherSummaryId, name: String, summary_type: WeatherSummaryType) -> Self {
        Self {
            id,
            name,
            summary_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeatherSummaryParams {
    pub header: WeatherSummaryHeader,
    pub geo: HashSet<LatticeEntry>,
    pub months: Vec<chrono::Month>,
    pub epoch: Epoch,
}

impl WeatherSummaryType {
    pub fn symbol(self) -> &'static str {
        match self {
            WeatherSummaryType::Wind => "💨",
        }
    }
}

impl std::fmt::Display for WeatherSummaryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            WeatherSummaryType::Wind => "Wind",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub enum WeatherSummaryData {
    Wind(Box<DirectionalIntensityHistogram<BeaufortScaleBucketer>>),
    Error(String),
    None,
}

impl From<Result<DirectionalIntensityHistogram<BeaufortScaleBucketer>, LoaderError>>
    for WeatherSummaryData
{
    fn from(
        value: Result<DirectionalIntensityHistogram<BeaufortScaleBucketer>, LoaderError>,
    ) -> Self {
        match value {
            Ok(data) => WeatherSummaryData::Wind(Box::new(data)),
            Err(e) => WeatherSummaryData::Error(e.to_string()),
        }
    }
}

impl WeatherSummaryData {
    pub fn stats(&self) -> Option<HistogramStats<'_>> {
        match self {
            WeatherSummaryData::Wind(histogram) => Some(histogram.stats()),
            _ => None,
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, WeatherSummaryData::None)
    }
}
