use std::{collections::HashMap, path::PathBuf};

use imma_files::{DownloadProgress, RemoteFileIndex, download_files};
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use url::Url;

pub async fn download_imma_files(start_year: i32, end_year: i32, output_dir: PathBuf) {
    let console = console::Term::stdout();
    if end_year < start_year {
        console
            .write_line("💥  End year must be greater than or equal to start year.")
            .expect("can write error");
        return;
    }

    let index_spinner = create_index_spinner();
    let index = match RemoteFileIndex::from_noaa().await {
        Ok(index) => index,
        Err(error) => {
            index_spinner.finish_and_clear();
            console
                .write_line(&format!("💥  {error}"))
                .expect("can write error");
            return;
        }
    };
    index_spinner.finish_and_clear();

    let files = index
        .iter_in_year_range(start_year..=end_year)
        .cloned()
        .collect::<Vec<_>>();

    if files.is_empty() {
        console
            .write_line(&format!(
                "No remote IMMA files found for years {start_year}..={end_year}."
            ))
            .expect("can write message");
        return;
    }

    let total_bytes = files.iter().map(|file| file.size).sum::<u64>();
    let urls = files
        .iter()
        .map(|file| file.url.clone())
        .collect::<Vec<_>>();
    let url_sizes = files
        .iter()
        .map(|file| (file.url.clone(), file.size))
        .collect::<HashMap<_, _>>();

    console
        .write_line(&format!(
            "Downloading {} files ({}) to {}.",
            urls.len(),
            HumanBytes(total_bytes),
            output_dir.display()
        ))
        .expect("can write message");

    let progress_bar = create_download_progress_bar(urls.len());
    let (progress_sender, mut progress_receiver) = tokio::sync::mpsc::unbounded_channel();
    let urls_to_download = urls.clone();
    let output_dir_clone = output_dir.clone();
    let mut download_handle = Some(tokio::spawn(async move {
        download_files(
            urls_to_download.as_slice(),
            output_dir_clone.as_path(),
            progress_sender,
        )
        .await
    }));

    let mut completed_files = 0usize;
    let mut completed_bytes = 0u64;
    let mut cancelled = false;

    loop {
        tokio::select! {
            progress = progress_receiver.recv() => {
                let Some(progress) = progress else {
                    break;
                };
                match progress {
                    DownloadProgress::Started(url) => {
                        progress_bar.set_message(format!("Downloading {}", display_file_name(&url)));
                    }
                    DownloadProgress::Retrying(url, error, attempt) => {
                        progress_bar.set_message(format!(
                            "Retrying {} (attempt {})",
                            display_file_name(&url),
                            attempt + 1
                        ));
                        progress_bar.println(format!(
                            "⚠️  Retry {} for {}: {}",
                            attempt,
                            display_file_name(&url),
                            error
                        ));
                    }
                    DownloadProgress::Complete(url) => {
                        completed_files += 1;
                        completed_bytes += url_sizes.get(&url).copied().unwrap_or(0);
                        progress_bar.inc(1);
                    }
                }
            }
            result = tokio::signal::ctrl_c() => {
                if result.is_ok() {
                    cancelled = true;
                    if let Some(download_handle) = download_handle.take() {
                        download_handle.abort();
                        let _ = download_handle.await;
                    }
                    break;
                }
            }
        }
    }

    if cancelled {
        progress_bar.finish_and_clear();
        console
            .write_line(&format!(
                "⚠️  Download cancelled after downloading {}/{} files ({}/{}) to {}.",
                completed_files,
                urls.len(),
                HumanBytes(completed_bytes),
                HumanBytes(total_bytes),
                output_dir.display()
            ))
            .expect("can write cancellation");
        return;
    }

    match download_handle.expect("download handle present").await {
        Ok(Ok(())) => {
            progress_bar.finish_and_clear();
            console
                .write_line(&format!(
                    "🎉 Downloaded {} files ({}) to {}.",
                    completed_files,
                    HumanBytes(total_bytes),
                    output_dir.display()
                ))
                .expect("can write success");
        }
        Ok(Err(error)) => {
            progress_bar.abandon_with_message("download failed");
            console
                .write_line(&format!("💥  {error}"))
                .expect("can write error");
        }
        Err(error) => {
            progress_bar.abandon_with_message("download failed");
            console
                .write_line(&format!("💥  {error}"))
                .expect("can write error");
        }
    }
}

fn create_download_progress_bar(file_count: usize) -> ProgressBar {
    let progress_bar = ProgressBar::new(file_count as u64);
    progress_bar.set_style(
        ProgressStyle::with_template("{elapsed} {bar:40.green/green} {pos}/{len} files {msg}")
            .expect("valid progress template")
            .progress_chars("##-"),
    );
    progress_bar
}

fn create_index_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .expect("valid spinner template")
            .tick_chars("|/-\\ "),
    );
    spinner.set_message("Fetching remote IMMA index...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner
}

fn display_file_name(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back().map(str::to_owned))
        .unwrap_or_else(|| url.as_str().to_owned())
}
