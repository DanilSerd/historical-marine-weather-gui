use arrow::array::ArrayBuilder;
use chrono::Utc;
use futures::{Stream, StreamExt};
use parquet::{
    arrow::{AsyncArrowWriter, async_writer::AsyncFileWriter},
    basic::Compression,
    errors::ParquetError,
    file::properties::WriterProperties,
};
use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
};
use tokio::task::JoinHandle;

use crate::DataVersion;
use crate::{data_file_prefix, traits::AsParquet};

const CHANNEL_SIZE: usize = 4;

/// The main parquet writer interface.
pub struct ParquetWriter {
    dir: PathBuf,
    options: WriterOptions,
    version: DataVersion,
}

impl ParquetWriter {
    /// Create a new writer, writing to dir.
    pub fn new(
        dir: &Path,
        options: WriterOptions,
        version: DataVersion,
    ) -> Result<Self, ParquetError> {
        dir.try_exists()
            .map_err(|_| {
                ParquetError::External("dir for writing parquet files is in unknown state".into())
            })?
            .then_some(())
            .ok_or(ParquetError::External(
                "dir for writing parquet files does not exist".into(),
            ))?;
        Ok(Self {
            options,
            dir: dir.to_path_buf(),
            version,
        })
    }

    /// Write from a stream to a new file(s).
    pub async fn write<S, P>(&self, mut stream: S) -> Result<ParquetFiles, ParquetError>
    where
        P: AsParquet,
        S: Stream<Item = P> + Unpin,
    {
        let files = ParquetFiles::new(&self.dir, self.version, self.options.number_of_files);
        let temp_files = files.create_temp_files().await?;

        let mut writer = PartitionedWriterBuilder::new(temp_files, self.options.clone()).await?;

        while let Some(item) = stream.next().await {
            writer.write(item).await?;
        }
        writer.finish().await?;

        Ok(files)
    }
}

struct PartitionBuilderHandle<P: AsParquet> {
    items: Vec<P>,
    handle: tokio::task::JoinHandle<Result<P::ArrayBuilderType, ParquetError>>,
    tx: tokio::sync::mpsc::Sender<Vec<P>>,
    count: usize,
    untouched_for_cycles: usize,
}

impl<P> PartitionBuilderHandle<P>
where
    P: AsParquet,
{
    fn new(capacity: usize) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let handle = tokio::spawn(start_builder(rx));
        Self {
            items: Vec::with_capacity(capacity),
            handle,
            tx,
            count: 0,
            untouched_for_cycles: 0,
        }
    }

    fn add(&mut self, item: P) {
        self.items.push(item);
        self.count += 1;
        self.untouched_for_cycles = 0;
    }

    async fn conditional_sendoff(
        &mut self,
        items_above_threshold: usize,
    ) -> Result<(), ParquetError> {
        if !self.items.is_empty() && self.items.len() >= items_above_threshold {
            let items = std::mem::take(&mut self.items);
            self.tx
                .send(items)
                .await
                .map_err(|_| ParquetError::External("builder send error".into()))?;
        }
        Ok(())
    }
}

/// Writes out [`AsParquet`] items to a parquet file. Ensures that items with the same partition end up in the same row group, upto batch_size in size.
struct PartitionedWriterBuilder<P, W>
where
    P: AsParquet,
{
    writers: Vec<WriterOrHandle<W>>,
    builder_handles: HashMap<P::Partition, PartitionBuilderHandle<P>>,
    cycle_count: usize,
    options: WriterOptions,
}

impl<P, W> PartitionedWriterBuilder<P, W>
where
    P: AsParquet,
    W: AsyncFileWriter + 'static,
    P::ArrayBuilderType: Send + Sync,
{
    async fn new(files: Vec<W>, options: WriterOptions) -> Result<Self, ParquetError> {
        if files.is_empty() {
            return Err(ParquetError::External("no files to write to".into()));
        }
        let schema = P::schema()?;
        let writers = files
            .into_iter()
            .map(|file| {
                AsyncArrowWriter::try_new(
                    file,
                    schema.clone(),
                    Some(options.build_parquet_properties()?),
                )
                .map(|writer| WriterOrHandle::Writer(Box::new(writer)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            writers,
            builder_handles: HashMap::new(),
            cycle_count: 0,
            options,
        })
    }

    async fn write(&mut self, item: P) -> Result<(), ParquetError> {
        let partition = item.partition();
        self.cycle_count += 1;
        if self.cycle_count >= self.options.max_batch_size {
            self.end_max_in_flight().await?;
            self.cycle_count = 0;
        }

        let builder_handle = self
            .builder_handles
            .entry(partition.clone())
            .or_insert_with(|| {
                PartitionBuilderHandle::new(self.options.parquet_page_row_count_limit)
            });

        // This is a bit of a strange pattern but sending batches results in much higher
        // throughput.
        builder_handle.add(item);
        builder_handle
            .conditional_sendoff(self.options.parquet_page_row_count_limit)
            .await?;

        if builder_handle.count >= self.options.max_batch_size {
            self.end(Some(vec![partition])).await?;
        }
        Ok(())
    }

    /// Check if max in flight was exceeeded and end builders in order of (untouched cycles ** 2) * count.
    /// Oldest and largest builders are ended first.
    async fn end_max_in_flight(&mut self) -> Result<(), ParquetError> {
        let overall_count = self
            .builder_handles
            .values()
            .fold(0, |acc, handle| acc + handle.count);
        if overall_count > self.options.max_in_flight {
            let mut all_handle_counts: Vec<_> = self
                .builder_handles
                .iter()
                .map(|(p, v)| (p, v.untouched_for_cycles, v.count))
                .collect();

            all_handle_counts.sort_by_key(|(_, u, c)| (*u + 1).pow(2) * c);
            all_handle_counts.reverse();

            let mut take_untill_count: i64 =
                (self.options.max_batch_size * 2).min(self.options.max_in_flight / 2) as i64;
            let partitions: Vec<_> = all_handle_counts
                .into_iter()
                .take_while(|(_, _, c)| {
                    let r = take_untill_count > 0;
                    take_untill_count -= (*c) as i64;
                    r
                })
                .map(|(p, _, _)| p.clone())
                .collect();
            self.end(Some(partitions)).await?;
        }

        self.builder_handles
            .values_mut()
            .for_each(|v| v.untouched_for_cycles += 1);

        Ok(())
    }

    async fn end(&mut self, partitions: Option<Vec<P::Partition>>) -> Result<(), ParquetError> {
        let mut handles: Vec<_> = match partitions {
            Some(pars) => pars
                .into_iter()
                .map(|p| {
                    self.builder_handles
                        .remove(&p)
                        .expect("partition builder handle found")
                })
                .collect(),
            None => self.builder_handles.drain().map(|h| h.1).collect(),
        };

        for h in handles.iter_mut() {
            h.conditional_sendoff(0).await?;
        }

        // This drops the channel sender which completes the builder future.
        // This drops them all together so they are finilizing in parallel.
        let handles = handles.into_iter().map(|h| h.handle);

        for handle in handles {
            let builder = handle
                .await
                .map_err(|_| ParquetError::External("builder join error".into()))??;
            self.get_writer().write::<P>(builder).await?;
        }

        Ok(())
    }

    async fn finish(mut self) -> Result<(), ParquetError> {
        self.end(None).await?;
        for writer in self.writers.iter_mut() {
            writer.finish().await?;
        }
        Ok(())
    }

    fn get_writer(&mut self) -> &mut WriterOrHandle<W> {
        let start = rand::random_range(..self.writers.len());

        let ready_i = (0..self.writers.len())
            .map(|offset| (start + offset) % self.writers.len())
            .find(|i| self.writers[*i].is_ready_for_write());

        match ready_i {
            Some(i) => &mut self.writers[i],
            None => &mut self.writers[0],
        }
    }
}

async fn start_builder<P: AsParquet>(
    mut channel: tokio::sync::mpsc::Receiver<Vec<P>>,
) -> Result<P::ArrayBuilderType, ParquetError> {
    let mut builder = P::new_array();
    while let Some(items) = channel.recv().await {
        for item in items {
            P::arrow_serialize(item.underlying_type(), &mut builder)?;
        }
    }
    Ok(builder)
}

#[derive(Default)]
enum WriterOrHandle<R> {
    Writer(Box<AsyncArrowWriter<R>>),
    Handle(JoinHandle<parquet::errors::Result<AsyncArrowWriter<R>>>),
    #[default]
    Empty,
}

impl<R> WriterOrHandle<R>
where
    R: AsyncFileWriter + 'static,
{
    /// This creates a new future that writes the batch if writer is available or await a handle which is using the writer and then spawns a new handle.
    async fn write<P: AsParquet>(
        &mut self,
        mut builder: P::ArrayBuilderType,
    ) -> parquet::errors::Result<()> {
        let wh = std::mem::take(self);
        match wh {
            WriterOrHandle::Writer(mut w) => {
                let h = tokio::spawn(async move {
                    w.write(&P::build_batch(builder.finish())?).await?;
                    w.flush().await?;
                    Ok(*w)
                });
                *self = WriterOrHandle::Handle(h);
            }
            WriterOrHandle::Handle(h) => {
                let mut w = h.await.map_err(|_| {
                    parquet::errors::ParquetError::External("writer handle join error".into())
                })??;
                let h = tokio::spawn(async move {
                    w.write(&P::build_batch(builder.finish())?).await?;
                    w.flush().await?;
                    Ok(w)
                });
                *self = WriterOrHandle::Handle(h);
            }
            WriterOrHandle::Empty => unreachable!("empty writer or handle"),
        };
        Ok(())
    }

    async fn finish(&mut self) -> parquet::errors::Result<()> {
        match self {
            WriterOrHandle::Writer(w) => {
                w.finish().await?;
            }
            WriterOrHandle::Handle(h) => {
                h.await
                    .map_err(|_| {
                        parquet::errors::ParquetError::External(
                            "writer handle join error on flush".into(),
                        )
                    })??
                    .finish()
                    .await?;
            }
            WriterOrHandle::Empty => unreachable!("empty writer or handle"),
        }
        Ok(())
    }

    fn is_ready_for_write(&self) -> bool {
        match self {
            WriterOrHandle::Writer(_) => true,
            WriterOrHandle::Handle(h) => h.is_finished(),
            WriterOrHandle::Empty => unreachable!("empty writer or handle"),
        }
    }
}

#[derive(Clone, Debug)]
/// Writer options for parquet.
pub struct WriterOptions {
    /// How many files to write to.
    pub number_of_files: usize,
    /// Maximum number of items to keep in flight. This is to limit the memory consumption of the writer.
    pub max_in_flight: usize,
    /// Batch size to construct before sending to parquet. This will be the size of each row group.
    /// Large values may impact memory.
    pub max_batch_size: usize,
    /// Compression to use.
    pub parquet_file_compression: CompressionCodec,
    /// Page size for the parquet files. You want to set low enough so when you read the files pages can be skipped.
    /// Optimal value is highly dependant on the way the data is queried and sorted.
    pub parquet_page_row_count_limit: usize,
}

impl Default for WriterOptions {
    fn default() -> Self {
        Self {
            number_of_files: 4,
            // TODO: This is just a hard coded limit just now. If the user has less memory
            // available then this will use, major slow down is likely. The user should be
            // dynamically updated of issues like this. And/or this limit should be dynamic based
            // on avaiable memory.
            max_in_flight: 1024 * 1024 * 8,
            max_batch_size: 1024 * 256,
            parquet_file_compression: CompressionCodec::SNAPPY,
            parquet_page_row_count_limit: 1024 * 16,
        }
    }
}

impl WriterOptions {
    fn build_parquet_properties(&self) -> Result<WriterProperties, ParquetError> {
        let properties = WriterProperties::builder()
            .set_compression(self.parquet_file_compression.into())
            .set_data_page_row_count_limit(self.parquet_page_row_count_limit)
            .set_write_batch_size(self.parquet_page_row_count_limit)
            .set_max_row_group_size(self.max_batch_size)
            .build();
        Ok(properties)
    }
}

/// Compression to use for the files.
#[derive(Clone, Debug, Copy)]
pub enum CompressionCodec {
    SNAPPY,
    GZIP,
}

impl From<CompressionCodec> for Compression {
    fn from(value: CompressionCodec) -> Self {
        match value {
            CompressionCodec::SNAPPY => Self::SNAPPY,
            CompressionCodec::GZIP => Self::GZIP(Default::default()),
        }
    }
}

impl Display for CompressionCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionCodec::SNAPPY => f.write_str("snappy"),
            CompressionCodec::GZIP => f.write_str("gzip"),
        }
    }
}

pub struct ParquetFiles(Vec<ParquetFile>);

impl ParquetFiles {
    fn new(dir: &Path, version: DataVersion, number_of_files: usize) -> Self {
        Self(
            (0..number_of_files)
                .map(|i| ParquetFile::new(dir, version, i))
                .collect(),
        )
    }

    async fn create_temp_files(&self) -> Result<Vec<tokio::fs::File>, ParquetError> {
        let mut files = Vec::with_capacity(self.0.len());
        for file in self.0.iter() {
            files.push(file.create_temp_file().await?);
        }
        Ok(files)
    }

    pub async fn commit(self) -> Result<(), ParquetError> {
        for file in self.0 {
            file.finalize().await?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ParquetFile {
    dir: PathBuf,
    file_name: String,
}

impl ParquetFile {
    fn new(dir: &Path, version: DataVersion, sequence: usize) -> Self {
        let created = Utc::now();
        Self {
            file_name: format!(
                "{}_{}_{}.parquet",
                data_file_prefix(version),
                created.format("%Y-%m-%d-%H-%M-%S"),
                sequence
            ),
            dir: dir.to_path_buf(),
        }
    }

    fn temp_file_path(&self) -> PathBuf {
        let mut path = self.dir.clone();
        path.push(format!("_{}", self.file_name));
        path
    }

    fn final_file_path(&self) -> PathBuf {
        let mut path = self.dir.clone();
        path.push(self.file_name.clone());
        path
    }

    fn delete_temp_file(&self) -> std::io::Result<()> {
        std::fs::remove_file(self.temp_file_path())
    }

    async fn create_temp_file(&self) -> std::io::Result<tokio::fs::File> {
        tokio::fs::File::options()
            .write(true)
            .append(false)
            .create(true)
            .truncate(true)
            .open(&self.temp_file_path())
            .await
    }

    async fn finalize(self) -> std::io::Result<()> {
        tokio::fs::rename(self.temp_file_path(), self.final_file_path()).await
    }
}

impl Drop for ParquetFile {
    fn drop(&mut self) {
        let _ = self.delete_temp_file();
    }
}
