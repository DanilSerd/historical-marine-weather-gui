use std::{
    fmt::Display, marker::PhantomData, path::Path, path::PathBuf, sync::Arc,
    thread::available_parallelism,
};

use arrow::{compute::concat_batches, util::pretty::pretty_format_batches};
use arrow_convert::deserialize::TryIntoCollection;
use datafusion::{
    datasource::{
        file_format::parquet::ParquetFormat,
        listing::{ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl},
    },
    error::DataFusionError,
    prelude::{SessionConfig, SessionContext},
};
use futures::{Stream, StreamExt, TryStreamExt};

use crate::{DataStats, DataVersion, data_file_prefix, traits::AsParquet};

const DEFAULT_TABLE_NAME: &str = "data";

/// Main type for reading parquet data.
pub struct ParquetReader<P> {
    ctx: SessionContext,
    tokio_runtime: Option<tokio::runtime::Runtime>,
    _phantom_p: PhantomData<P>,
    pub version: DataVersion,
    pub dir: PathBuf,
}

impl<P> ParquetReader<P>
where
    P: AsParquet + 'static,
    P::Type: Send,
{
    /// Create a new reader, reading from dir.
    ///
    /// If runtime is not provided, the reader will assume it's running in context of tokio.
    pub fn new(
        dir: &Path,
        version: DataVersion,
        runtime: Option<tokio::runtime::Runtime>,
    ) -> Result<Self, DataFusionError> {
        let config = SessionConfig::new()
            .set_bool("datafusion.execution.parquet.pushdown_filters", true)
            .set_bool("datafusion.execution.parquet.reorder_filters", true);
        // TODO Make sure caching of the metadata is working. For that to work we need datafusion crate > 49.0.0. (not released at the time of writing)
        // TODO: .set_bool("datafusion.execution.parquet.cache_metadata", true);
        let ctx = SessionContext::new_with_config(config);
        register_table::<P>(dir, version, DEFAULT_TABLE_NAME, &ctx)?;
        Ok(Self {
            ctx,
            tokio_runtime: runtime,
            _phantom_p: PhantomData,
            version,
            dir: dir.to_path_buf(),
        })
    }

    /// Read based on [`AsParquet::Predicate`].
    pub async fn read(
        &self,
        predicate_projection: &P::Predicate,
    ) -> Result<impl Stream<Item = Result<P::Type, DataFusionError>> + use<P>, DataFusionError>
    {
        match &self.tokio_runtime {
            Some(runtime) => runtime.block_on(self.read_internal(predicate_projection)),
            None => self.read_internal(predicate_projection).await,
        }
    }

    pub async fn explain(
        &self,
        verbose: bool,
        analyze: bool,
        predicate_projection: &P::Predicate,
    ) -> Result<impl Display, DataFusionError> {
        match &self.tokio_runtime {
            Some(runtime) => {
                runtime.block_on(self.explain_internal(verbose, analyze, predicate_projection))
            }
            None => {
                self.explain_internal(verbose, analyze, predicate_projection)
                    .await
            }
        }
    }

    async fn read_internal(
        &self,
        predicate_projection: &P::Predicate,
    ) -> Result<impl Stream<Item = Result<P::Type, DataFusionError>> + use<P>, DataFusionError>
    {
        let df = self.ctx.table(DEFAULT_TABLE_NAME).await?;
        let df = P::read(predicate_projection, df)?;

        let df_stream = df.execute_stream().await?;
        let s = df_stream
            .map(move |b| async move {
                let b = b?;
                let array = P::build_array(b)?;
                let desered: Vec<P::Type> = array
                    .try_into_collection_as_type::<P>()
                    .expect("can construct collection from array");
                Result::<_, DataFusionError>::Ok(futures::stream::iter(desered).map(Ok))
            })
            .buffered(available_parallelism().unwrap().get())
            .try_flatten();
        Ok(s)
    }

    async fn explain_internal(
        &self,
        verbose: bool,
        analyze: bool,
        predicate_projection: &P::Predicate,
    ) -> Result<impl Display, DataFusionError> {
        let df = self.ctx.table(DEFAULT_TABLE_NAME).await?;
        let df = P::read(predicate_projection, df)?;
        let explain = df.explain(verbose, analyze)?.execute_stream().await?;
        let explain: Vec<_> = explain.try_collect().await?;
        Ok(pretty_format_batches(&explain)?)
    }
}

impl<P> ParquetReader<P>
where
    P: DataStats,
{
    pub async fn stats(&self) -> Result<Option<P::Stats>, DataFusionError> {
        let df = self.ctx.table(DEFAULT_TABLE_NAME).await?;
        let df = P::build(df)?;
        let result = df.collect().await?;
        let Some(first) = result.first() else {
            return Ok(None);
        };
        let batch = concat_batches(&first.schema(), result.iter())?;
        let stats = P::stats(batch)?;
        Ok(stats)
    }
}

fn register_table<P: AsParquet>(
    dir: &Path,
    version: DataVersion,
    table: &str,
    ctx: &SessionContext,
) -> datafusion::error::Result<()> {
    let table_glob = format!(
        "{}/{}*",
        dir.to_str().expect("dir is path"),
        data_file_prefix(version)
    );
    let table_path = ListingTableUrl::parse(&table_glob)?;

    let listing_options = ListingOptions::new(Arc::new(ParquetFormat::new()))
        .with_file_extension(".parquet")
        .with_collect_stat(true);

    let config = ListingTableConfig::new(table_path)
        .with_listing_options(listing_options)
        .with_schema(P::schema()?);

    let provider = Arc::new(ListingTable::try_new(config)?);

    ctx.register_table(table, provider)?;

    Ok(())
}
