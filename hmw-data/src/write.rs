use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use futures::{StreamExt, pin_mut};
use hmw_parquet::{ParquetFiles, ParquetWriter};
use imma_files::{FileSource, read_file_sources};

use super::error::Error;
use super::types::MarineWeatherObservation;

const UPDATE_PROCESSED_SO_FAR_EVERY: usize = 256;
const SEND_UPDATE_EVERY_MS: u64 = 100;

#[derive(Debug)]
pub enum Progress {
    /// Number of files to process. This comes first.
    FilesToProcess(usize),
    /// Number of records processed so far. Emitted periodically.
    ProcessedSoFar(usize),
    /// The file just started.
    Started(FileSource),
    /// The file is complete.
    Complete(FileSource),
    /// All files are complete.
    AllFilesComplete,
    /// Conversion is complete and the number of items processed is specified.
    ConversionComplete(usize),
    /// The imma file had an error.
    Error((FileSource, Error)),
}

pub async fn process_imma_data_to_parquet(
    imma_files_and_dirs: &[FileSource],
    writer: &ParquetWriter,
    read_parallelism: usize,
    progress_channel: tokio::sync::mpsc::UnboundedSender<Progress>,
) -> Result<ParquetFiles, Error> {
    let (files_to_process, imma_stream) = read_file_sources(imma_files_and_dirs, read_parallelism)
        .await
        .map_err(imma_files::Error::IOError)?;

    let _ = progress_channel.send(Progress::FilesToProcess(files_to_process));

    let progress_channel_clone = progress_channel.clone();

    let items_processed = Arc::new(AtomicUsize::new(0));
    let tick_channel = progress_channel.clone();
    let tick_ip = items_processed.clone();
    let tick_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(SEND_UPDATE_EVERY_MS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let _ = tick_channel.send(Progress::ProcessedSoFar(
                tick_ip.load(std::sync::atomic::Ordering::Relaxed),
            ));
        }
    });

    let mut files_processed: usize = 0;
    let files_processed_ref = &mut files_processed;
    let mut items_processed_counter: usize = 0;
    let items_processed_counter_ref = &mut items_processed_counter;

    let imma_stream = imma_stream
        .map(move |imma_r| {
            let result = match imma_r {
                Ok(f) => match f {
                    imma_files::FileRecord::Started(path_buf) => {
                        let _ = progress_channel.send(Progress::Started(path_buf));
                        None
                    }
                    imma_files::FileRecord::Record(immarecord) => {
                        let wor = MarineWeatherObservation::new_from_imma(immarecord);
                        *items_processed_counter_ref += 1;
                        if (*items_processed_counter_ref)
                            .is_multiple_of(UPDATE_PROCESSED_SO_FAR_EVERY)
                        {
                            items_processed.store(
                                *items_processed_counter_ref,
                                std::sync::atomic::Ordering::Relaxed,
                            );
                        }
                        Some(wor)
                    }
                    imma_files::FileRecord::Complete(path_buf) => {
                        let _ = progress_channel.send(Progress::Complete(path_buf));
                        *files_processed_ref += 1;
                        None
                    }
                },
                Err(e) => {
                    let _ = progress_channel.send(Progress::Error((e.file, e.error.into())));
                    *files_processed_ref += 1;
                    None
                }
            };

            if *files_processed_ref >= files_to_process {
                let _ = progress_channel.send(Progress::AllFilesComplete);
            }

            result
        })
        .filter_map(std::future::ready);

    pin_mut!(imma_stream);

    let files = writer.write(imma_stream).await?;

    let _ = progress_channel_clone.send(Progress::ConversionComplete(items_processed_counter));
    tick_handle.abort();

    Ok(files)
}
