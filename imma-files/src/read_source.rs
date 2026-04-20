use std::{collections::HashSet, io::Read, thread::sleep, time::Duration};

use crate::{
    error::{Error, FileError},
    source::FileSource,
};
use async_stream::stream;
use flate2::read::GzDecoder;
use futures::{Stream, StreamExt, TryStreamExt};
use imma_parser::{
    iter::{FinishingIter, IMMAReadIteratorBuilder},
    types::IMMARecord,
};
use tokio::{sync::mpsc, task::JoinSet};
use tokio_stream::wrappers::ReadDirStream;
const CHANNEL_CAPACITY: usize = 1024;

/// Read local or remote IMMA file sources into a stream.
pub async fn read_file_sources(
    sources: &[FileSource],
    parallel_reads: usize,
) -> Result<(usize, impl Stream<Item = Result<FileRecord, FileError>>), std::io::Error> {
    let non_existant_or_no_permission = sources
        .iter()
        .filter_map(FileSource::missing_local_path)
        .collect::<Vec<_>>();

    if non_existant_or_no_permission.len() == sources.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No paths exist or you do not have permission to read them",
        ));
    }

    let (mut files_to_process, processing_stream) =
        expand_and_read_sources(sources, parallel_reads).await?;
    files_to_process += non_existant_or_no_permission.len();

    let stream = if !non_existant_or_no_permission.is_empty() {
        let non_existent_stream = stream! {
            for source in non_existant_or_no_permission {
                yield Err(FileError {
                    file: source,
                    error: Error::IOError(std::io::Error::new(std::io::ErrorKind::NotFound, "Path does not exist or you do not have permission to read it")),
                });
            }
        };
        non_existent_stream.chain(processing_stream).boxed()
    } else {
        processing_stream.boxed()
    };

    Ok((files_to_process, stream))
}

enum InputReader {
    Local(std::fs::File),
    LocalGzip(GzDecoder<std::fs::File>),
    Remote(reqwest::blocking::Response),
    RemoteGzip(GzDecoder<reqwest::blocking::Response>),
}

impl Read for InputReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        match self {
            Self::Local(reader) => reader.read(buf),
            Self::LocalGzip(reader) => reader.read(buf),
            Self::Remote(reader) => reader.read(buf),
            Self::RemoteGzip(reader) => reader.read(buf),
        }
    }
}

/// File record to track progress of reading of directory.
#[derive(Debug)]
pub enum FileRecord {
    /// File started to be processed.
    Started(FileSource),
    /// IMMA record from some file.
    Record(IMMARecord),
    /// File is complete.
    Complete(FileSource),
}

async fn expand_and_read_sources(
    sources: &[FileSource],
    parallel_reads: usize,
) -> Result<(usize, impl Stream<Item = Result<FileRecord, FileError>>), std::io::Error> {
    let mut files = HashSet::new();
    for source in sources {
        match source {
            FileSource::Local(path) if path.is_dir() => {
                let dir_stream = ReadDirStream::new(tokio::fs::read_dir(path).await?);
                let dir_files: Vec<_> = dir_stream
                    .try_filter_map(|entry| async {
                        let f = entry;
                        let path = f.path();
                        let is_dir = f.metadata().await?.is_dir();
                        if is_dir {
                            return Ok(None);
                        }
                        Ok(Some(FileSource::Local(path)))
                    })
                    .try_collect()
                    .await?;
                files.extend(dir_files);
            }
            FileSource::Local(path) => {
                files.insert(FileSource::Local(path.clone()));
            }
            FileSource::Remote(url) => {
                files.insert(FileSource::Remote(url.clone()));
            }
        }
    }
    let mut files: Vec<_> = files.into_iter().collect();
    files.sort();
    let files_to_process = files.len();

    let stream = stream! {
        let (tx, mut rx) = mpsc::channel(parallel_reads * CHANNEL_CAPACITY);
        let h = tokio::spawn(async move {
            let mut js: JoinSet<()> = tokio::task::JoinSet::new();
            for file in files {
                if js.len() >= parallel_reads {
                    let _ = js.join_next().await;
                }

                let tx_clone = tx.clone();
                js.spawn_blocking(|| {
                    read_source_file_outer(file, tx_clone);
                });
            }
            join_all_or_ignore_cancellation(js).await;
        });
        let mut buffer = Vec::with_capacity(parallel_reads * CHANNEL_CAPACITY);
        loop {
            // recv_many performs much better than just receiving one by one.
            let recieved_count = rx.recv_many(&mut buffer, parallel_reads * CHANNEL_CAPACITY).await;
            if recieved_count == 0 {
                break;
            }
            for r in buffer.drain(..) {
                yield r;
            }

        }
        let _ = h.await;
    };

    Ok((files_to_process, stream))
}

async fn join_all_or_ignore_cancellation(mut join_set: JoinSet<()>) {
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(()) => (),
            Err(error) if error.is_cancelled() => (),
            Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
            Err(_) => (),
        }
    }
}

fn read_source_file_outer(source: FileSource, out: mpsc::Sender<Result<FileRecord, FileError>>) {
    if out
        .blocking_send(Ok(FileRecord::Started(source.clone())))
        .is_err()
    {
        return;
    }

    let result = read_source_file(source.clone(), out.clone());
    match result {
        Ok(_) => {
            let _ = out.blocking_send(Ok(FileRecord::Complete(source.clone())));
        }
        Err(error) => {
            let _ = out.blocking_send(Err(FileError {
                file: source,
                error,
            }));
        }
    }
}

fn read_source_file(
    source: FileSource,
    out: mpsc::Sender<Result<FileRecord, FileError>>,
) -> Result<(), Error> {
    let is_gz = match source.extension() {
        Some("gz" | "gzip" | "GZIP") => true,
        Some(_) => return Err(Error::UnsoportedFileExtention),
        None => false,
    };
    let read_records = |reader, processed_records: &mut usize| {
        let already_processed = *processed_records;
        let mut iter = IMMAReadIteratorBuilder::new(reader).build();
        for _ in iter.by_ref().take(already_processed) {}
        for record in iter.by_ref() {
            out.blocking_send(Ok(FileRecord::Record(record)))?;
            *processed_records += 1;
        }
        iter.finish().map_err(Error::from)
    };

    match source {
        FileSource::Local(path) => {
            let input_file = std::fs::File::options().read(true).open(path)?;
            let reader = match is_gz {
                true => InputReader::LocalGzip(GzDecoder::new(input_file)),
                false => InputReader::Local(input_file),
            };
            let mut processed_records = 0;
            read_records(reader, &mut processed_records)?;
        }
        FileSource::Remote((url, _)) => {
            let mut processed_records = 0;
            let mut backoff = Duration::from_secs(1);
            loop {
                let reader = match reqwest::blocking::Client::new()
                    .get(url.clone())
                    .send()
                    .and_then(reqwest::blocking::Response::error_for_status)
                {
                    Ok(response) => match is_gz {
                        true => InputReader::RemoteGzip(GzDecoder::new(response)),
                        false => InputReader::Remote(response),
                    },
                    Err(error) => {
                        if error.is_builder() {
                            return Err(Error::HttpError(error));
                        }
                        sleep(backoff);
                        backoff = (backoff * 2).min(Duration::from_secs(30));
                        // TODO: Report the transient error to the channel.
                        continue;
                    }
                };

                match read_records(reader, &mut processed_records) {
                    Ok(()) => break,
                    Err(Error::IMMAIterError(imma_parser::iter::Error::IoError(_))) => {
                        sleep(backoff);
                        backoff = (backoff * 2).min(Duration::from_secs(30));
                        // TODO: Report the transient error to the channel.
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }
    Result::<(), Error>::Ok(())
}
