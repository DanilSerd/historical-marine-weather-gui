use std::{path::PathBuf, time::Instant};

use console::style;
use hmw_data::{FileSource, ParquetWriter, WriterOptions, process_imma_data_to_parquet};
use imma_files::RemoteFileIndex;
use indicatif::{HumanBytes, HumanCount, ProgressBar, ProgressStyle};
use url::Url;

pub enum ProcessInput {
    Local(Vec<PathBuf>),
    RemoteYears { start_year: i32, end_year: i32 },
}

pub async fn process_imma(
    input: ProcessInput,
    output_dir: PathBuf,
    number_of_files: usize,
    max_batch_size: usize,
    max_in_flight: usize,
    parquet_page_row_count_limit: usize,
) {
    let console = console::Term::stdout();

    let sources = match input {
        ProcessInput::Local(paths) => {
            console
                .write_line(&format!(
                    "Processing local IMMA sources into {}.",
                    output_dir.display()
                ))
                .expect("can write message");
            paths.into_iter().map(FileSource::Local).collect::<Vec<_>>()
        }
        ProcessInput::RemoteYears {
            start_year,
            end_year,
        } => {
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
            console
                .write_line(&format!(
                    "Processing {} files ({}) into {}.",
                    files.len(),
                    HumanBytes(total_bytes),
                    output_dir.display()
                ))
                .expect("can write message");

            files
                .into_iter()
                .map(|file| FileSource::Remote(file.url))
                .collect::<Vec<_>>()
        }
    };

    let writer = match ParquetWriter::new(
        &output_dir,
        WriterOptions {
            number_of_files,
            max_batch_size,
            parquet_page_row_count_limit,
            max_in_flight,
            ..Default::default()
        },
        Default::default(),
    ) {
        Ok(writer) => writer,
        Err(error) => {
            console
                .write_line(&format!("💥  {error}"))
                .expect("can write error");
            return;
        }
    };

    let (progress_sender, mut progress_receiver) =
        tokio::sync::mpsc::unbounded_channel::<hmw_data::Progress>();
    let mut processing_handle = Some(tokio::spawn(async move {
        process_imma_data_to_parquet(
            sources.as_slice(),
            &writer,
            std::thread::available_parallelism().unwrap().get(),
            progress_sender,
        )
        .await?
        .commit()
        .await?;
        Ok::<(), hmw_data::Error>(())
    }));

    let mut files_progress: Option<ProgressBar> = None;
    let mut finalizing_spinner: Option<ProgressBar> = None;
    let mut total_files: Option<usize> = None;
    let mut successful_files = 0usize;
    let mut failed_files = 0usize;
    let mut records_written = 0usize;
    let mut processing_started_at: Option<Instant> = None;
    let mut cancelled = false;

    loop {
        tokio::select! {
            progress = progress_receiver.recv() => {
                let Some(progress) = progress else {
                    break;
                };
                match progress {
                    hmw_data::Progress::FilesToProcess(files) => {
                        total_files = Some(files);
                        processing_started_at = Some(Instant::now());
                        let progress_bar = create_files_progress_bar(files);
                        progress_bar.set_message(format_records_progress_message(0, processing_started_at));
                        files_progress = Some(progress_bar);
                    }
                    hmw_data::Progress::ProcessedSoFar(count) => {
                        if let Some(progress_bar) = files_progress.as_ref() {
                            progress_bar.set_message(format_records_progress_message(count, processing_started_at));
                        }
                    }
                    hmw_data::Progress::Started(source) => {
                        if let Some(progress_bar) = files_progress.as_ref() {
                            progress_bar.println(format!("⌛  {}", display_source_name(&source)));
                        }
                    }
                    hmw_data::Progress::AllFilesComplete => {
                        if let Some(progress_bar) = files_progress.take() {
                            progress_bar.finish_and_clear();
                        }
                        if finalizing_spinner.is_none() {
                            finalizing_spinner = Some(create_spinner("Finalizing..."));
                        }
                    }
                    hmw_data::Progress::Complete(source) => {
                        successful_files += 1;
                        if let Some(progress_bar) = files_progress.as_ref() {
                            progress_bar.println(format!("✅  {}", display_source_name(&source)));
                            progress_bar.inc(1);
                        }
                    }
                    hmw_data::Progress::ConversionComplete(count) => {
                        records_written = count;
                        if let Some(progress_bar) = files_progress.as_ref() {
                            progress_bar.set_message(format_records_progress_message(count, processing_started_at));
                        }
                    }
                    hmw_data::Progress::Error((source, error)) => {
                        failed_files += 1;
                        if let Some(progress_bar) = files_progress.as_ref() {
                            progress_bar.println(format!(
                                "❌  Failed {}: {}",
                                display_source_name(&source),
                                style(error.to_string()).red()
                            ));
                            progress_bar.inc(1);
                        } else {
                            console
                                .write_line(&format!(
                                    "❌  Failed {}: {}",
                                    display_source_name(&source),
                                    style(error.to_string()).red()
                                ))
                                .expect("can write error");
                        }
                    },
                }
            }
            result = tokio::signal::ctrl_c() => {
                if result.is_ok() {
                    cancelled = true;
                    if let Some(processing_handle) = processing_handle.take() {
                        processing_handle.abort();
                        let _ = processing_handle.await;
                    }
                    break;
                }
            }
        }
    }

    if cancelled {
        if let Some(progress_bar) = files_progress.take() {
            progress_bar.finish_and_clear();
        }
        if let Some(spinner) = finalizing_spinner.take() {
            spinner.finish_and_clear();
        }
        let total_files = total_files.map_or_else(|| "?".to_owned(), |files| files.to_string());
        console
            .write_line(&format!(
                "⚠️  Processing cancelled after finishing {}/{} files. Nothing was written.",
                successful_files + failed_files,
                total_files
            ))
            .expect("can write cancellation");
        return;
    }

    let processing_result = processing_handle.expect("processing handle present").await;

    if let Some(progress_bar) = files_progress.take() {
        progress_bar.finish_and_clear();
    }
    if let Some(spinner) = finalizing_spinner.take() {
        spinner.finish_and_clear();
    }

    match processing_result {
        Ok(Ok(())) => {
            let total_files = total_files.unwrap_or(successful_files + failed_files);
            if failed_files == 0 {
                console
                    .write_line(&format!(
                        "🎉 Processed {} files into {}.",
                        total_files,
                        output_dir.display()
                    ))
                    .expect("can write success");
            } else {
                console
                    .write_line(&format!(
                        "⚠️  Processed {}/{} files into {}. {} failed.",
                        successful_files,
                        total_files,
                        output_dir.display(),
                        failed_files
                    ))
                    .expect("can write summary");
            }
            console
                .write_line(&format!("Records written: {}", records_written))
                .expect("can write records written");
        }
        Ok(Err(error)) => {
            console
                .write_line(&format!("💥  {error}"))
                .expect("can write error");
        }
        Err(error) => {
            console
                .write_line(&format!("💥  {error}"))
                .expect("can write error");
        }
    }
}

fn create_files_progress_bar(file_count: usize) -> ProgressBar {
    let progress_bar = ProgressBar::new(file_count as u64);
    progress_bar.set_style(
        ProgressStyle::with_template("{pos}/{len} {bar:40.green/green} {msg} (ETA: {eta})")
            .expect("valid progress template")
            .progress_chars("##-"),
    );
    progress_bar
}

fn format_records_progress_message(count: usize, started_at: Option<Instant>) -> String {
    let human_count = HumanCount(count as u64);
    let rate = started_at
        .map(|started_at| started_at.elapsed().as_secs_f64())
        .filter(|elapsed| *elapsed > 0.0)
        .map(|elapsed| count as f64 / elapsed)
        .unwrap_or(0.0) as u64;

    format!("{human_count} records ({}/s)", HumanCount(rate))
}

fn create_index_spinner() -> ProgressBar {
    create_spinner("Fetching remote IMMA index...")
}

fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .expect("valid spinner template")
            .tick_chars("|/-\\ "),
    );
    spinner.set_message(message.to_owned());
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner
}

fn display_source_name(source: &FileSource) -> String {
    match source {
        FileSource::Local(path) => path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
        FileSource::Remote(url) => display_url_file_name(url),
    }
}

fn display_url_file_name(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back().map(str::to_owned))
        .unwrap_or_else(|| url.as_str().to_owned())
}
