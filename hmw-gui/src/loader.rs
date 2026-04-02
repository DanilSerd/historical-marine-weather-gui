use std::{fmt, sync::Arc};

use crate::types::WeatherSummaryParams;
use hmw_data::{
    DataReader, DataVersion, DirectionalBucketing, DirectionalIntensityHistogram, GetDate, GetTime,
    GetYear, LatticeFilter, MarineWeatherObservationDataStats, MonthFilter, Project,
};

#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error(transparent)]
    Data(#[from] hmw_data::Error),
    #[error("No data found")]
    Empty,
}

#[derive(Clone)]
pub struct LoaderStats {
    pub data_stats: Option<MarineWeatherObservationDataStats>,
}

#[derive(Clone)]
pub struct Loader {
    reader: Arc<DataReader>,
    stats: LoaderStats,
}

impl Loader {
    pub async fn new(
        data_dir: &std::path::Path,
        version: DataVersion,
    ) -> Result<Self, LoaderError> {
        let reader = DataReader::new(data_dir, version, None)?;
        let data_stats = reader.data_stats().await?;
        Ok(Self {
            reader: Arc::new(reader),
            stats: LoaderStats { data_stats },
        })
    }

    pub async fn load_directional_histogram<B>(
        &self,
        params: &WeatherSummaryParams,
    ) -> Result<DirectionalIntensityHistogram<B>, LoaderError>
    where
        B: DirectionalBucketing,
        B::Observation: Project + GetDate + GetTime + GetYear,
    {
        let lattice_selection = LatticeFilter::Lattice(params.geo.iter().cloned().collect());
        let months = MonthFilter::Months(params.months.clone());

        #[cfg(debug_assertions)]
        let mut explain_string = String::new();
        #[cfg(debug_assertions)]
        let explain = Some((true, true, &mut explain_string));
        #[cfg(not(debug_assertions))]
        let explain = None;

        let stream = self
            .reader
            .read::<B::Observation>(&lattice_selection, &params.epoch, &months, explain)
            .await?;

        let histogram = DirectionalIntensityHistogram::<B>::populate(stream).await?;
        #[cfg(debug_assertions)]
        {
            use tokio::io::AsyncWriteExt;

            let now = chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S").to_string();
            let file = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(false)
                .truncate(true)
                .open(format!("hmw-gui/.debug/reader_explain_{}", now))
                .await;
            match file {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(explain_string.as_bytes()).await {
                        dbg!(e);
                    }
                }
                Err(e) => {
                    dbg!(e);
                }
            }
        }
        if histogram.stats().histogram_counters.inserted == 0 {
            return Err(LoaderError::Empty);
        }
        Ok(histogram)
    }

    pub fn stats(&self) -> &LoaderStats {
        &self.stats
    }

    pub fn data_version(&self) -> DataVersion {
        self.reader.data_version()
    }
}

impl fmt::Debug for Loader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Loader")
    }
}
