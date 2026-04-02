use std::fmt::Display;

use futures::{Stream, TryStreamExt};
use ndhistogram::{
    AxesTuple, Histogram, VecHistogram, axis::CategoryNoFlow, ndhistogram, value::Sum,
};
use serde::{Serialize, Serializer};

use crate::{
    HistogramStats,
    error::Error,
    histogram::{
        date_time_histogram::{DateTimeHistogram, GetDate, GetTime, GetYear},
        stats::HistogramCounters,
    },
};

use super::{CardinalOrdinalDirection, DirectionalBucketing};

/// 2D histogram with cardinal ordinal direction and intensity.
#[derive(Debug, Clone)]
pub struct DirectionalIntensityHistogram<IA> {
    counts: VecHistogram<AxesTuple<(CategoryNoFlow<CardinalOrdinalDirection>, IA)>, Sum<usize>>,
    date_time: DateTimeHistogram,
    pub counters: HistogramCounters,
}

impl<IA> DirectionalIntensityHistogram<IA>
where
    IA: DirectionalBucketing,
{
    /// Populate the histogram from a stream.
    pub async fn populate<S>(mut stream: S) -> Result<Self, Error>
    where
        S: Stream<Item = Result<IA::Observation, Error>> + Unpin,
        IA::Observation: GetDate + GetTime + GetYear,
    {
        let mut counts = ndhistogram!(
            CategoryNoFlow::new(CardinalOrdinalDirection::all_cardinal_directions().iter().copied().chain(std::iter::once(CardinalOrdinalDirection::Indeterminate))),
            IA::default();
            Sum<usize>
        );

        let mut date_time = DateTimeHistogram::new();

        let mut counters = HistogramCounters::default();

        while let Some(observation) = stream.try_next().await? {
            let processed = match counters.add(IA::process(&observation)) {
                Some(p) => p,
                None => {
                    continue;
                }
            };
            counts.fill(&processed);
            date_time.fill(&observation);
        }

        Ok(Self {
            counts,
            date_time,
            counters,
        })
    }

    pub fn iter_non_empty<'s>(
        &'s self,
    ) -> impl Iterator<Item = DirectionalIntensity<IA::BinInterval>> + use<'s, IA>
    where
        IA::BinInterval: Display,
    {
        let num_obs = self.counters.inserted;
        self.counts.iter().filter_map(move |count_bin| {
            let count_bin_val = count_bin.value.sum();
            if count_bin_val == 0 {
                return None;
            }
            let (db, ib) = count_bin.bin;
            let d = *db.value().expect("no overflow for direction");
            Some(DirectionalIntensity {
                intensity_bucket: ib,
                direction_bucket: d,
                probability: count_bin_val as f64 / num_obs as f64,
                count: count_bin.value.sum(),
            })
        })
    }

    pub fn stats(&self) -> HistogramStats<'_> {
        HistogramStats {
            date_time: &self.date_time,
            histogram_counters: &self.counters,
        }
    }
}

/// Probability of the wind returned from the histogram.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct DirectionalIntensity<SB: Display> {
    /// The intensity bucket.
    #[serde(serialize_with = "serialize_as_str")]
    pub intensity_bucket: SB,
    /// The direction bucket.
    pub direction_bucket: CardinalOrdinalDirection,
    /// Probability of this bucket combination. count/overall count
    pub probability: f64,
    /// Count of observations in the bucket.
    pub count: usize,
}

fn serialize_as_str<S, SB: Display>(speed_bucket: &SB, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(speed_bucket.to_string().as_str())
}
