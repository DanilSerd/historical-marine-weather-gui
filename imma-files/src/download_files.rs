use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use futures_util::TryStreamExt;
use tokio::{
    fs::{self, File},
    io::{self, AsyncWriteExt},
    sync::mpsc,
};
use tokio_util::io::StreamReader;
use url::Url;

use crate::Error;

struct PendingDownloadFile {
    temp_path: PathBuf,
    final_path: PathBuf,
    file: Option<File>,
    persisted: bool,
}

impl PendingDownloadFile {
    async fn new(output_dir: &Path, file_name: &str) -> Result<Self, Error> {
        let final_file_name = file_name.strip_prefix('_').unwrap_or(file_name);
        let temp_file_name = format!("_{final_file_name}");
        let mut temp_path = output_dir.to_path_buf();
        temp_path.push(temp_file_name);
        let mut final_path = output_dir.to_path_buf();
        final_path.push(final_file_name);
        let file = File::create(&temp_path).await?;

        Ok(Self {
            temp_path,
            final_path,
            file: Some(file),
            persisted: false,
        })
    }

    fn file_mut(&mut self) -> &mut File {
        self.file.as_mut().expect("pending file is present")
    }

    async fn persist(mut self) -> Result<(), Error> {
        self.file_mut().flush().await?;
        self.file_mut().sync_all().await?;
        let _ = self.file.take();
        fs::rename(&self.temp_path, &self.final_path).await?;
        self.persisted = true;
        Ok(())
    }
}

impl Drop for PendingDownloadFile {
    fn drop(&mut self) {
        if !self.persisted {
            let _ = std::fs::remove_file(&self.temp_path);
        }
    }
}

/// Progress events emitted while downloading remote files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadProgress {
    /// A file download has started.
    Started(Url),
    /// A file download failed and will be retried.
    Retrying(Url, String, usize),
    /// A file download has completed.
    Complete(Url),
}

/// Download files sequentially into the provided output directory.
pub async fn download_files(
    urls: &[Url],
    output_dir: &Path,
    progress_channel: mpsc::UnboundedSender<DownloadProgress>,
) -> Result<(), Error> {
    fs::create_dir_all(output_dir).await?;
    let client = reqwest::Client::new();

    for url in urls {
        progress_channel.send(DownloadProgress::Started(url.clone()))?;

        let mut retry_count = 0usize;
        let mut retry_delay = Duration::from_secs(1);
        loop {
            match download_file(&client, url, output_dir).await {
                Ok(()) => {
                    progress_channel.send(DownloadProgress::Complete(url.clone()))?;
                    break;
                }
                Err(error) => {
                    retry_count += 1;
                    progress_channel.send(DownloadProgress::Retrying(
                        url.clone(),
                        error.to_string(),
                        retry_count,
                    ))?;
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = std::cmp::min(retry_delay * 2, Duration::from_secs(30));
                }
            }
        }
    }

    Ok(())
}

async fn download_file(
    client: &reqwest::Client,
    url: &Url,
    output_dir: &Path,
) -> Result<(), Error> {
    let file_name = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| Error::RemoteDownloadError(format!("missing file name in URL {url}")))?;

    let response = client.get(url.clone()).send().await?.error_for_status()?;
    let stream = response.bytes_stream().map_err(io::Error::other);
    let mut input = StreamReader::new(stream);
    let mut output_file = PendingDownloadFile::new(output_dir, file_name).await?;

    tokio::io::copy(&mut input, output_file.file_mut()).await?;

    output_file.persist().await?;
    Ok(())
}
