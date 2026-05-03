use std::{collections::HashSet, fmt::Display};

use hmw_data::{BeaufortScaleBucketer, DirectionalIntensityHistogram, Epoch, HistogramStats};
use hmw_geo::LatticeEntry;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::loader::Loader;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum WeatherSummary {
    Wind(WeatherSummaryWithKind<WindSummaryKind>),
}

/// Summary with kind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSummaryWithKind<K: WeatherSummaryKind> {
    params: WeatherSummaryParams,
    #[serde(skip)]
    data: WeatherSummaryData<K::Histogram>,
}

/// Parameters describing the summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeatherSummaryParams {
    pub header: WeatherSummaryHeader,
    pub geo: HashSet<LatticeEntry>,
    pub months: Vec<chrono::Month>,
    pub epoch: Epoch,
}

/// Data of the summary, aka the histogram or error.
#[derive(Debug, Clone, Default)]
pub enum WeatherSummaryData<H> {
    Data(Box<H>),
    Error(String),
    #[default]
    None,
}

pub trait WeatherSummaryKind {
    type Histogram;

    const ENUM: WeatherSummaryKindEnum;
}

/// Enum representing the summary kinds. Each weathersummary kind maps to one of these.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, strum::EnumIter, Default)]
pub enum WeatherSummaryKindEnum {
    #[default]
    Wind,
}

/// A header for a weather summary. Includes a unique identifier and a name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeatherSummaryHeader {
    pub id: WeatherSummaryId,
    pub name: String,
}

/// Unique ID for each summary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WeatherSummaryId(uuid::Uuid);

impl WeatherSummary {
    pub fn new(params: WeatherSummaryParams, kind: WeatherSummaryKindEnum) -> Self {
        match kind {
            WeatherSummaryKindEnum::Wind => Self::Wind(WeatherSummaryWithKind {
                params,
                data: Default::default(),
            }),
        }
    }

    pub async fn populate_data(self, loader: Loader) -> Self {
        match self {
            WeatherSummary::Wind(s) => WeatherSummary::Wind(s.populate_data(loader).await),
        }
    }

    pub fn kind_enum(&self) -> WeatherSummaryKindEnum {
        match self {
            WeatherSummary::Wind(w) => w.kind_enum(),
        }
    }

    pub fn params(&self) -> &WeatherSummaryParams {
        match self {
            WeatherSummary::Wind(w) => w.params(),
        }
    }

    pub fn data_avaialble(&self) -> Result<bool, &str> {
        match self {
            WeatherSummary::Wind(w) => w.data_avaialble(),
        }
    }

    pub fn data_stats(&self) -> Option<HistogramStats<'_>> {
        match self {
            WeatherSummary::Wind(w) => w.data_stats(),
        }
    }

    pub fn invalidate_data(&mut self) {
        match self {
            WeatherSummary::Wind(w) => w.invalidate_data(),
        }
    }
}

impl<K> WeatherSummaryWithKind<K>
where
    K: WeatherSummaryKind,
{
    pub fn kind_enum(&self) -> WeatherSummaryKindEnum {
        K::ENUM
    }

    pub fn params(&self) -> &WeatherSummaryParams {
        &self.params
    }

    pub fn data_avaialble(&self) -> Result<bool, &str> {
        match &self.data {
            WeatherSummaryData::Data(_) => Ok(true),
            WeatherSummaryData::Error(e) => Err(e),
            WeatherSummaryData::None => Ok(false),
        }
    }

    pub fn data(&self) -> Option<&K::Histogram> {
        match &self.data {
            WeatherSummaryData::Data(d) => Some(d),
            WeatherSummaryData::Error(_) => None,
            WeatherSummaryData::None => None,
        }
    }

    pub fn invalidate_data(&mut self) {
        self.data = WeatherSummaryData::None
    }
}

// TODO: This might be better done by having a trait on the histogram types.
impl WeatherSummaryWithKind<WindSummaryKind> {
    pub fn data_stats(&self) -> Option<HistogramStats<'_>> {
        match &self.data {
            WeatherSummaryData::Data(d) => Some(d.stats()),
            WeatherSummaryData::Error(_) => None,
            WeatherSummaryData::None => None,
        }
    }

    pub async fn populate_data(mut self, loader: Loader) -> Self {
        let result = loader.load_directional_histogram(&self.params).await;
        match result {
            Ok(d) => self.data = WeatherSummaryData::Data(Box::new(d)),
            Err(e) => self.data = WeatherSummaryData::Error(e.to_string()),
        };
        self
    }
}

impl WeatherSummaryKindEnum {
    pub fn name(&self) -> &'static str {
        match self {
            WeatherSummaryKindEnum::Wind => "Wind",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            WeatherSummaryKindEnum::Wind => "💨",
        }
    }

    pub fn all_kinds() -> impl Iterator<Item = Self> {
        WeatherSummaryKindEnum::iter()
    }
}

impl Display for WeatherSummaryKindEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone)]
pub struct WindSummaryKind;

impl WeatherSummaryKind for WindSummaryKind {
    type Histogram = DirectionalIntensityHistogram<BeaufortScaleBucketer>;

    const ENUM: WeatherSummaryKindEnum = WeatherSummaryKindEnum::Wind;
}

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
