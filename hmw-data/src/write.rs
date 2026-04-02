use futures::{StreamExt, pin_mut};
use hmw_parquet::{ParquetFiles, ParquetWriter};
use imma_files::{FileSource, read_file_sources};

use super::error::Error;
use super::types::MarineWeatherObservation;

const REPORT_PROCESSED_SO_FAR_EVERY: usize = 10000;

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

    let mut items_processed: usize = 0;
    let mut files_processed: usize = 0;
    let items_processed_ref = &mut items_processed;
    let files_processed_ref = &mut files_processed;
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
                        *items_processed_ref += 1;
                        if (*items_processed_ref).is_multiple_of(REPORT_PROCESSED_SO_FAR_EVERY) {
                            let _ = progress_channel
                                .send(Progress::ProcessedSoFar(*items_processed_ref));
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

    let _ = progress_channel_clone.send(Progress::ConversionComplete(items_processed));

    Ok(files)
}
