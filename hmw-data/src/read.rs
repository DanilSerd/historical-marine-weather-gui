use std::{collections::HashSet, path::Path};

use futures::{Stream, StreamExt};
use hmw_parquet::{DataVersion, ParquetReader};

use crate::{
    Epoch, LatticeFilter, MarineWeatherObservation, MarineWeatherObservationDataStats, MonthFilter,
    Project, error::Error,
};

macro_rules! ready_ok {
    ($e:expr) => {
        std::future::ready(Some(Ok($e)))
    };
}

macro_rules! ready_err {
    ($e:expr) => {
        std::future::ready(Some(Err($e)))
    };
}

macro_rules! ready_none {
    () => {
        std::future::ready(None)
    };
}

pub struct DataReader {
    reader: ParquetReader<MarineWeatherObservation>,
}

impl DataReader {
    pub fn new(
        dir: &Path,
        version: DataVersion,
        runtime: Option<tokio::runtime::Runtime>,
    ) -> Result<Self, Error> {
        let reader = ParquetReader::<MarineWeatherObservation>::new(dir, version, runtime)?;
        Ok(Self { reader })
    }

    pub async fn read<P: Project>(
        &self,
        lattice_filter: &LatticeFilter,
        epoch: &Epoch,
        months: &MonthFilter,
        with_explain: Option<(bool, bool, &mut String)>,
    ) -> Result<impl Stream<Item = Result<P, Error>>, Error> {
        let predicate_projection =
            MarineWeatherObservation::predicate::<P>(epoch, months, lattice_filter);

        if let Some((verbose, analyze, out)) = with_explain {
            let explain = self
                .reader
                .explain(verbose, analyze, &predicate_projection)
                .await?;
            *out = explain.to_string();
        };

        // Using this hashset to deduplicate based on uid.
        let mut uids = HashSet::new();
        let stream = self
            .reader
            .read(&predicate_projection)
            .await?
            .filter_map(move |r| {
                let mut o = match r {
                    Ok(o) => o,
                    Err(e) => return ready_err!(e.into()),
                };
                let uid = o
                    .attm_uida
                    .take()
                    .expect("uid should exist on each observation");
                if uids.insert(uid.uid) {
                    ready_ok!(o.project())
                } else {
                    ready_none!()
                }
            });

        Ok(stream)
    }

    pub fn data_version(&self) -> DataVersion {
        self.reader.version
    }

    pub async fn data_stats(&self) -> Result<Option<MarineWeatherObservationDataStats>, Error> {
        self.reader.stats().await.map_err(Error::from)
    }

    pub fn data_dir(&self) -> &Path {
        &self.reader.dir
    }
}
