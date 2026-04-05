use std::mem;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::Datelike;
use iced::task::Handle;
use iced::widget::{
    button, column, container, progress_bar, row, scrollable, text, text_input, toggler,
};
use iced::{Alignment, Element, Font, Length, Subscription, Task, font::Weight};

use hmw_data::{
    Error as DataError, FileSource, ParquetWriter, Progress as ImportProgress, WriterOptions,
    process_imma_data_to_parquet,
};
use imma_files::RemoteFileIndex;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::collapsable::Collapsible;
use crate::utils::icon_widget;
use crate::widgets::{AnimatedEllipsis, follow_tooltip};

const NOAA_START_YEAR_MIN: i32 = 1662;
const NOAA_FINAL_DATA_END_YEAR: i32 = 2014;
const BUTTON_HEIGHT: f32 = 36.0;
const SOURCE_CONTROLS_HEIGHT: f32 = 40.0;
const WRITER_OPTIONS_LABEL_WIDTH: f32 = 220.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    SelectParquetDir,
    ImportImma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Controls whether sources are added from disk or from the NOAA index.
pub(crate) enum ImportSourceMode {
    LocalFiles,
    #[default]
    NoaaDownload,
}

/// Identifies which NOAA year input field changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoaaYearField {
    Start,
    End,
}

#[derive(Debug, Clone)]
struct NoaaYearRangeInput {
    start: String,
    end: String,
}

impl NoaaYearRangeInput {
    fn new() -> Self {
        Self {
            start: NOAA_START_YEAR_MIN.to_string(),
            end: NOAA_FINAL_DATA_END_YEAR.to_string(),
        }
    }

    fn parsed(&self) -> Option<RangeInclusive<i32>> {
        let start = parse_noaa_year(&self.start)?;
        let end = parse_noaa_year(&self.end)?;

        match end >= start {
            true => Some(start..=end),
            false => None,
        }
    }

    fn update(&mut self, field: NoaaYearField, input: String) {
        match field {
            NoaaYearField::Start => self.start = digits_only(input),
            NoaaYearField::End => self.end = digits_only(input),
        }
    }

    fn validation_error(&self) -> Option<String> {
        let start = parse_noaa_year(&self.start);
        let end = parse_noaa_year(&self.end);

        match (start, end) {
            (Some(start), Some(end)) if end < start => {
                Some("End year must be greater than or equal to start year.".to_string())
            }
            (Some(_), Some(_)) => None,
            _ => Some(format!(
                "Years must be between {} and {}.",
                NOAA_START_YEAR_MIN,
                current_year()
            )),
        }
    }

    fn nrt_warning(&self) -> Option<String> {
        self.parsed()
            .filter(|years| *years.end() > NOAA_FINAL_DATA_END_YEAR)
            .map(|_| {
                format!(
                    "Warning: years later than {} use NRT (near realtime) data.",
                    NOAA_FINAL_DATA_END_YEAR
                )
            })
    }
}

#[derive(Debug, Clone, Default)]
pub enum Message {
    // Mode switching
    SwitchMode(Mode),
    UpdateDialogueStatus(DialogueOpenStatusMessage),

    // Mode 1: select parquet dir
    PickParquetDir,
    ApplyParquetDir(PathBuf),

    // Mode 2: import flow controls
    AddSourceFiles,
    SourceFilesPicked(Vec<PathBuf>),
    AddSourceDir,
    SourceDirPicked(Option<PathBuf>),
    RemoveSource(FileSource),
    ClearAllSources,
    SelectImportSourceMode(ImportSourceMode),
    UpdateNoaaYearRange(NoaaYearField, String),
    FetchNoaaSources,
    FetchedNoaaSources(Result<RemoteFileIndex, String>),
    ToggleAdvancedSettingsCollapsable(bool),
    UpdateWriterOptions(WriterOptions),
    StartImport,

    // Progress streaming
    ProgressFilesToProcess(usize),
    ProgressFileStatus(FileStatusEntry),
    ProgressAllFilesComplete,
    ProgressItemsProcessed(usize),
    ProgressFinished(Result<(), String>),
    AnimationTick,
    ProgressReset,
    #[default]
    None,
}

impl Message {
    pub fn applied_parquet_dir(&self) -> Option<&Path> {
        match self {
            Message::ApplyParquetDir(path) => Some(path),
            _ => None,
        }
    }
}

pub struct DataFileManager {
    dialogue_open_status: DialogueOpenStatus,
    // Mode
    mode: Mode,

    selected_parquet_dir: Option<PathBuf>,

    // Mode 2: import
    import_source_mode: ImportSourceMode,
    sources: Vec<FileSource>,
    noaa_year_range: NoaaYearRangeInput,
    cached_noaa_index: Option<RemoteFileIndex>,
    noaa_fetch_error: Option<String>,
    fetching_noaa_index: bool,
    files_import: Vec<FileStatusEntry>,
    total_files: usize,
    all_files_complete: bool,
    total_items_processed: usize,
    import_started_at: Option<Instant>,
    status_ellipsis: AnimatedEllipsis,
    advanced_settings_expanded: bool,
    writer_options: WriterOptions,
    processing_outcome: Option<Result<(), String>>,

    import_handles: Vec<Handle>,
}

impl DataFileManager {
    pub fn new(selected_data_dir: Option<PathBuf>) -> Self {
        Self {
            dialogue_open_status: Default::default(),
            mode: Mode::SelectParquetDir,
            selected_parquet_dir: selected_data_dir,
            import_source_mode: Default::default(),
            sources: Default::default(),
            noaa_year_range: NoaaYearRangeInput::new(),
            cached_noaa_index: Default::default(),
            noaa_fetch_error: Default::default(),
            fetching_noaa_index: Default::default(),
            files_import: Default::default(),
            total_files: Default::default(),
            all_files_complete: Default::default(),
            total_items_processed: Default::default(),
            import_started_at: Default::default(),
            status_ellipsis: Default::default(),
            processing_outcome: Default::default(),
            advanced_settings_expanded: Default::default(),
            writer_options: Default::default(),
            import_handles: Default::default(),
        }
    }

    pub fn selected_data_dir(&self) -> Option<&Path> {
        self.selected_parquet_dir.as_deref()
    }

    /// Returns the subscriptions needed by the import progress view.
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        self.status_ellipsis
            .subscription(self.is_animating())
            .map(|()| Message::AnimationTick)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::None => Task::none(),
            Message::SwitchMode(mode) => {
                self.mode = mode;
                Task::none()
            }
            Message::UpdateDialogueStatus(status_message) => {
                self.dialogue_open_status.toggle(status_message);
                Task::none()
            }
            Message::PickParquetDir => {
                let dialogue = rfd::AsyncFileDialog::new().set_title("Select Data Directory");
                let fut = dialogue.pick_folder();
                let task = Task::future(fut).map(|o| {
                    o.map(|h| Message::ApplyParquetDir(h.path().to_path_buf()))
                        .unwrap_or_default()
                });
                Task::done(Message::UpdateDialogueStatus(
                    DialogueOpenStatusMessage::DataDir(true),
                ))
                .chain(task)
                .chain(Task::done(Message::UpdateDialogueStatus(
                    DialogueOpenStatusMessage::DataDir(false),
                )))
            }
            Message::ApplyParquetDir(path) => {
                self.selected_parquet_dir = Some(path);
                Task::none()
            }
            Message::AddSourceFiles => {
                let dialogue = rfd::AsyncFileDialog::new().set_title("Select Source Files");
                let fut = dialogue.pick_files();
                let task = Task::future(fut).map(|v| {
                    Message::SourceFilesPicked(
                        v.unwrap_or_default()
                            .iter()
                            .map(|h| h.path().to_path_buf())
                            .collect(),
                    )
                });
                Task::done(Message::UpdateDialogueStatus(
                    DialogueOpenStatusMessage::ImportFiles(true),
                ))
                .chain(task)
                .chain(Task::done(Message::UpdateDialogueStatus(
                    DialogueOpenStatusMessage::ImportFiles(false),
                )))
            }
            Message::SourceFilesPicked(files) => {
                self.insert_sources(files.into_iter().map(FileSource::Local));
                Task::none()
            }
            Message::AddSourceDir => {
                let dialogue = rfd::AsyncFileDialog::new().set_title("Select Source Directory");
                let fut = dialogue.pick_folder();
                let task = Task::future(fut)
                    .map(|o| Message::SourceDirPicked(o.map(|h| h.path().to_path_buf())));
                Task::done(Message::UpdateDialogueStatus(
                    DialogueOpenStatusMessage::ImportFolder(true),
                ))
                .chain(task)
                .chain(Task::done(Message::UpdateDialogueStatus(
                    DialogueOpenStatusMessage::ImportFolder(false),
                )))
            }
            Message::SourceDirPicked(path) => {
                if let Some(path) = path {
                    self.insert_source(FileSource::Local(path));
                }
                Task::none()
            }
            Message::RemoveSource(source) => {
                self.sources
                    .retain(|existing_source| existing_source != &source);
                Task::none()
            }
            Message::ClearAllSources => {
                self.sources.clear();
                Task::none()
            }
            Message::SelectImportSourceMode(import_source_mode) => {
                self.import_source_mode = import_source_mode;
                self.noaa_fetch_error = None;
                Task::none()
            }
            Message::UpdateNoaaYearRange(field, input) => {
                self.noaa_year_range.update(field, input);
                self.noaa_fetch_error = None;
                Task::none()
            }
            Message::FetchNoaaSources => {
                let years = match self.noaa_year_range.parsed() {
                    Some(years) => years,
                    None => {
                        self.noaa_fetch_error = self.noaa_year_range.validation_error();
                        return Task::none();
                    }
                };

                if self.cached_noaa_index.is_some() {
                    self.noaa_fetch_error = self.add_noaa_sources_in_range(years).err();
                    return Task::none();
                }

                self.fetching_noaa_index = true;
                self.noaa_fetch_error = None;
                self.status_ellipsis.reset();

                Task::future(async move {
                    RemoteFileIndex::from_noaa()
                        .await
                        .map_err(|error| error.to_string())
                })
                .map(Message::FetchedNoaaSources)
            }
            Message::FetchedNoaaSources(result) => {
                self.fetching_noaa_index = false;
                self.status_ellipsis.reset();

                match result {
                    Ok(index) => {
                        self.cached_noaa_index = Some(index);
                        self.noaa_fetch_error = self
                            .noaa_year_range
                            .parsed()
                            .ok_or_else(|| {
                                self.noaa_year_range.validation_error().unwrap_or_default()
                            })
                            .and_then(|years| self.add_noaa_sources_in_range(years))
                            .err();
                    }
                    Err(error) => {
                        self.noaa_fetch_error = Some(error);
                    }
                }
                Task::none()
            }
            Message::ToggleAdvancedSettingsCollapsable(is_expanded) => {
                self.advanced_settings_expanded = is_expanded;
                Task::none()
            }
            Message::UpdateWriterOptions(options) => {
                self.writer_options = options;
                Task::none()
            }
            Message::StartImport => {
                self.files_import.clear();
                self.total_files = 0;
                self.all_files_complete = false;
                self.total_items_processed = 0;
                self.import_started_at = Some(Instant::now());
                self.status_ellipsis.reset();
                self.processing_outcome = None;

                let (out, sources) = match (
                    mem::take(&mut self.sources),
                    self.selected_parquet_dir.clone(),
                ) {
                    (sources, Some(destination)) if !sources.is_empty() => (destination, sources),
                    (sources, _) => {
                        self.sources = sources;
                        return Task::none();
                    }
                };

                let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<ImportProgress>();

                let writer_options = self.writer_options.clone();
                let fut = async move {
                    // TODO: Do something with this version.
                    let version = Default::default();

                    let writer = ParquetWriter::new(&out, writer_options, version)?;
                    let files = process_imma_data_to_parquet(
                        &sources,
                        &writer,
                        // TODO: Make this configurable by the user.
                        std::thread::available_parallelism()
                            .map(|n| n.get())
                            .unwrap_or(4),
                        tx,
                    )
                    .await?;

                    files.commit().await?;
                    Ok(())
                };

                let (processing_task, processing_handle) = Task::future(fut)
                    .map(|r| Message::ProgressFinished(r.map_err(|e: DataError| e.to_string())))
                    .abortable();
                let progress_stream = UnboundedReceiverStream::new(rx);
                let (progress_task, progress_handle) =
                    Task::run(progress_stream, Message::from).abortable();
                self.import_handles = vec![
                    processing_handle.abort_on_drop(),
                    progress_handle.abort_on_drop(),
                ];

                Task::batch([processing_task, progress_task])
            }
            Message::ProgressFilesToProcess(f) => {
                self.total_files = f;
                Task::none()
            }
            Message::ProgressFileStatus(file_status_entry) => {
                let index = self
                    .files_import
                    .iter()
                    .position(|f| f.path == file_status_entry.path);
                match index {
                    Some(index) => self.files_import[index] = file_status_entry,
                    None => self.files_import.push(file_status_entry),
                }
                Task::none()
            }
            Message::ProgressAllFilesComplete => {
                self.all_files_complete = true;
                self.status_ellipsis.reset();
                Task::none()
            }
            Message::ProgressItemsProcessed(i) => {
                self.total_items_processed = i;
                Task::none()
            }
            Message::ProgressFinished(r) => {
                self.processing_outcome = Some(r);
                self.status_ellipsis.reset();

                // Firing off this message so we can update the loader/collections.
                Task::done(
                    self.selected_data_dir()
                        .as_ref()
                        .map(|p| Message::ApplyParquetDir(p.to_path_buf()))
                        .unwrap_or_default(),
                )
            }
            Message::AnimationTick => {
                self.status_ellipsis.tick();
                Task::none()
            }
            Message::ProgressReset => {
                self.import_handles.clear();
                self.files_import.clear();
                self.total_files = 0;
                self.all_files_complete = false;
                self.total_items_processed = 0;
                self.import_started_at = None;
                self.status_ellipsis.reset();
                self.processing_outcome = None;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let switch = row([
            self.view_mode_tab("Data Directory", Mode::SelectParquetDir),
            self.view_mode_tab("Import ICOADS", Mode::ImportImma),
        ])
        .spacing(2)
        .padding([0, 3]);

        let body: Element<'_, Message> = match self.mode {
            Mode::SelectParquetDir => self.view_select_parquet_dir(),
            Mode::ImportImma => self.view_import_imma(),
        };

        column([
            container(switch).into(),
            container(body)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .padding(10)
        .into()
    }

    fn view_mode_tab(&self, label: &'static str, mode: Mode) -> Element<'_, Message> {
        let active = self.mode == mode;

        button(text(label).size(15))
            .padding([8, 16])
            .height(35)
            .style(move |theme, status| style::mode_tab(theme, status, active))
            .on_press(Message::SwitchMode(mode))
            .into()
    }

    fn view_select_parquet_dir(&self) -> Element<'_, Message> {
        column([self.view_selected_dir_row("Data directory...")])
            .spacing(10)
            .into()
    }

    fn view_selected_dir_row(&self, placeholder: &'static str) -> Element<'_, Message> {
        let input = text_input(
            placeholder,
            &self
                .selected_parquet_dir
                .as_ref()
                .map(|p| p.to_string_lossy())
                .unwrap_or_default(),
        )
        .padding(8)
        .size(16)
        .width(Length::Fill);

        let pick = action_button(
            "Pick Folder",
            Some("Pick the folder used to store data"),
            (!self.dialogue_open_status.select_data_dir).then_some(Message::PickParquetDir),
            button::primary,
        );

        row([input.into(), pick])
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
    }

    fn view_import_imma(&self) -> Element<'_, Message> {
        match self.import_handles.is_empty() {
            true => self.view_import_imma_selection(),
            false => self.view_import_imma_progress(),
        }
    }

    fn view_import_imma_selection(&self) -> Element<'_, Message> {
        let mut sources_list = column([]).spacing(4);
        for s in &self.sources {
            sources_list = sources_list.push(view_source_row(s));
        }
        if self.sources.is_empty() {
            sources_list = sources_list.push(text("No sources selected."));
        }

        let scrollable = scrollable(container(sources_list).padding([6, 8]).width(Length::Fill))
            .style(style::file_list_scrollable)
            .height(Length::Fill)
            .width(Length::Fill);

        let clear_all_sources = action_button(
            "Remove All",
            Some("Remove all source files"),
            (!self.sources.is_empty()).then_some(Message::ClearAllSources),
            button::danger,
        );

        let sources_header = row([
            text("Sources")
                .font(Font {
                    weight: Weight::Bold,
                    ..Font::DEFAULT
                })
                .into(),
            container(text("")).width(Length::Fill).into(),
            clear_all_sources,
        ])
        .spacing(8)
        .align_y(Alignment::Center);

        let sources_row = match self.import_source_mode {
            ImportSourceMode::LocalFiles => self.view_local_source_controls(),
            ImportSourceMode::NoaaDownload => self.view_noaa_source_controls(),
        };

        let dest_row = self.view_selected_dir_row("Destination folder...");

        let start_import = action_button(
            "Import",
            None,
            (!self.sources.is_empty()
                && self.selected_parquet_dir.is_some()
                && !self.fetching_noaa_index
                && !writer_options_unavailable(&self.writer_options))
            .then_some(Message::StartImport),
            button::primary,
        );

        let advanced_settings = Collapsible::new(
            text("Advanced Settings"),
            writer_options_view(&self.writer_options),
            self.advanced_settings_expanded,
            Message::ToggleAdvancedSettingsCollapsable,
        );

        column([
            sources_header.into(),
            scrollable.into(),
            self.view_import_source_switch(),
            sources_row,
            dest_row,
            advanced_settings.into(),
            start_import,
        ])
        .spacing(10)
        .into()
    }

    fn view_import_source_switch(&self) -> Element<'_, Message> {
        let is_noaa_download = self.import_source_mode == ImportSourceMode::NoaaDownload;

        row([
            source_mode_label("Local Files", !is_noaa_download),
            toggler(is_noaa_download)
                .on_toggle(|is_noaa_download| {
                    Message::SelectImportSourceMode(match is_noaa_download {
                        true => ImportSourceMode::NoaaDownload,
                        false => ImportSourceMode::LocalFiles,
                    })
                })
                .into(),
            source_mode_label("NOAA Download", is_noaa_download),
        ])
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_local_source_controls(&self) -> Element<'_, Message> {
        row([
            action_button(
                "+ Files",
                Some("Add one or more local ICOADS files"),
                (!self.dialogue_open_status.select_import_files).then_some(Message::AddSourceFiles),
                button::primary,
            ),
            action_button(
                "+ Folder",
                Some("Add all local ICOADS files from a folder"),
                (!self.dialogue_open_status.select_import_folder).then_some(Message::AddSourceDir),
                button::primary,
            ),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .height(Length::Fixed(SOURCE_CONTROLS_HEIGHT))
        .into()
    }

    fn view_noaa_source_controls(&self) -> Element<'_, Message> {
        let add_button = action_button(
            "+ Add Files",
            Some("Add ICOADS files for the selected year range"),
            (self.can_fetch_noaa_sources() && !self.fetching_noaa_index)
                .then_some(Message::FetchNoaaSources),
            button::primary,
        );

        let fetch_status: Element<'_, Message> = match (
            self.fetching_noaa_index,
            self.noaa_year_range.validation_error(),
            self.noaa_fetch_error.as_ref(),
            self.noaa_year_range.nrt_warning(),
        ) {
            (true, _, _, _) => text(self.status_ellipsis.text("Fetching NOAA index")).into(),
            (false, Some(error), _, _) => text(error).style(iced::widget::text::danger).into(),
            (false, None, Some(error), _) => text(error).style(iced::widget::text::danger).into(),
            (false, None, None, Some(warning)) => text(warning).style(warning_text_style).into(),
            (false, None, None, None) => container(text(""))
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into(),
        };

        row([
            text_input("Start year", &self.noaa_year_range.start)
                .on_input(|input| Message::UpdateNoaaYearRange(NoaaYearField::Start, input))
                .padding(8)
                .width(Length::Fixed(110.0))
                .into(),
            text_input("End year", &self.noaa_year_range.end)
                .on_input(|input| Message::UpdateNoaaYearRange(NoaaYearField::End, input))
                .padding(8)
                .width(Length::Fixed(110.0))
                .into(),
            add_button,
            container(fetch_status).width(Length::Fill).into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center)
        .height(Length::Fixed(SOURCE_CONTROLS_HEIGHT))
        .into()
    }

    fn view_import_imma_progress(&self) -> Element<'_, Message> {
        let mut files_column = column([]).spacing(4);
        for f in &self.files_import {
            let icon = match &f.status {
                FileStatusKind::Started => icon_widget("⏳"),
                FileStatusKind::Success => icon_widget("✔"),
                FileStatusKind::Error(_) => icon_widget("❌"),
            };
            let mut roww = row([icon.into(), source_name_view(&f.path)])
                .spacing(6)
                .align_y(Alignment::Center);
            if let FileStatusKind::Error(e) = &f.status {
                roww = roww.push(text(format!(" - {}", e)));
            }
            files_column = files_column.push(roww);
        }

        let success_count = self
            .files_import
            .iter()
            .filter(|f| f.status == FileStatusKind::Success)
            .count();
        let failed_count = self
            .files_import
            .iter()
            .filter(|f| matches!(f.status, FileStatusKind::Error(_)))
            .count();

        if self.processing_outcome.is_some() {
            if success_count > 0 {
                files_column = files_column.push(
                    row([
                        icon_widget("✔").into(),
                        text(format!("Succeeded: {} files", success_count)).into(),
                    ])
                    .spacing(6),
                );
            }
            if failed_count > 0 {
                files_column = files_column.push(
                    row([
                        icon_widget("❌").into(),
                        text(format!("Failed: {} files", failed_count)).into(),
                    ])
                    .spacing(6),
                );
            }
            if let Some(Err(error)) = self.processing_outcome.as_ref() {
                files_column = files_column.push(
                    row([
                        icon_widget("❌").into(),
                        text(format!("Error: {}", error))
                            .style(iced::widget::text::danger)
                            .into(),
                    ])
                    .spacing(6),
                );
            }
        }

        let processed_files = self
            .files_import
            .iter()
            .filter(|f| matches!(f.status, FileStatusKind::Success | FileStatusKind::Error(_)))
            .count();
        let progress_value = match self.total_files {
            0 => 0.0,
            total_files => processed_files as f32 / total_files as f32,
        };
        let records_progress_text = {
            let rate = self
                .import_started_at
                .map(|started_at| started_at.elapsed().as_secs_f64())
                .filter(|elapsed| *elapsed > 0.0)
                .map(|elapsed| self.total_items_processed as f64 / elapsed)
                .unwrap_or(0.0);

            let formater = human_format::Formatter::new();
            format!(
                "{} records\n({}/s)",
                formater.format(self.total_items_processed as f64),
                formater.format(rate)
            )
        };

        let progress_row = match self.processing_outcome.as_ref() {
            Some(Ok(())) => row([
                container(text(format!(
                    "Done: {} records",
                    human_format::Formatter::new().format(self.total_items_processed as f64)
                )))
                .width(Length::Fill)
                .into(),
                action_button("Ok", None, Some(Message::ProgressReset), button::primary),
            ])
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
            Some(Err(_)) => row([
                container(text("Failed: output not finalized").style(iced::widget::text::danger))
                    .width(Length::Fill)
                    .into(),
                action_button("Ok", None, Some(Message::ProgressReset), button::primary),
            ])
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
            None => {
                let mut progress = progress_bar(0.0..=1.0, progress_value);

                if self.all_files_complete {
                    progress = progress.style(iced::widget::progress_bar::success);
                }

                let progress_text =
                    text(format!("{}/{}", processed_files, self.total_files)).into();
                let progress_status: Element<'_, Message> = if self.all_files_complete {
                    container(text(self.status_ellipsis.text("Finalizing"))).into()
                } else {
                    container(text(records_progress_text).width(Length::Fill)).into()
                };

                row([
                    progress_text,
                    container(progress).width(Length::Fill).into(),
                    container(progress_status).width(Length::Fixed(150.)).into(),
                    action_button(
                        "Cancel",
                        Some("Stop the current import. All progress will be lost."),
                        Some(Message::ProgressReset),
                        button::danger,
                    ),
                ])
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            }
        };

        column([
            container(
                scrollable(container(files_column).padding([6, 8]).width(Length::Fill))
                    .style(style::file_list_scrollable)
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .anchor_bottom(),
            )
            .into(),
            progress_row,
        ])
        .spacing(10)
        .into()
    }
}

impl From<ImportProgress> for Message {
    fn from(p: ImportProgress) -> Self {
        match p {
            ImportProgress::FilesToProcess(n) => Message::ProgressFilesToProcess(n),
            ImportProgress::ProcessedSoFar(n) => Message::ProgressItemsProcessed(n),
            ImportProgress::Started(p) => {
                Message::ProgressFileStatus(FileStatusEntry::new_with_started(p))
            }
            ImportProgress::Complete(p) => {
                Message::ProgressFileStatus(FileStatusEntry::new_with_success(p))
            }
            ImportProgress::AllFilesComplete => Message::ProgressAllFilesComplete,
            ImportProgress::ConversionComplete(n) => Message::ProgressItemsProcessed(n),
            ImportProgress::Error((p, e)) => {
                Message::ProgressFileStatus(FileStatusEntry::new_with_error(p, e.to_string()))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum FileStatusKind {
    Started,
    Success,
    Error(String),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) struct FileStatusEntry {
    path: FileSource,
    status: FileStatusKind,
}

impl FileStatusEntry {
    fn new_with_success(path: FileSource) -> Self {
        Self {
            path,
            status: FileStatusKind::Success,
        }
    }

    fn new_with_error(path: FileSource, error: String) -> Self {
        Self {
            path,
            status: FileStatusKind::Error(error),
        }
    }

    fn new_with_started(path: FileSource) -> Self {
        Self {
            path,
            status: FileStatusKind::Started,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DialogueOpenStatusMessage {
    DataDir(bool),
    ImportFolder(bool),
    ImportFiles(bool),
}

#[derive(Debug, Clone, Default)]
struct DialogueOpenStatus {
    select_data_dir: bool,
    select_import_folder: bool,
    select_import_files: bool,
}

impl DialogueOpenStatus {
    fn toggle(&mut self, message: DialogueOpenStatusMessage) {
        match message {
            DialogueOpenStatusMessage::DataDir(on) => {
                self.select_data_dir = on;
            }
            DialogueOpenStatusMessage::ImportFolder(on) => {
                self.select_import_folder = on;
            }
            DialogueOpenStatusMessage::ImportFiles(on) => {
                self.select_import_files = on;
            }
        };
    }
}

mod style {
    use iced::border::Radius;
    use iced::widget::{container, scrollable};
    use iced::{Background, Border, Color, Theme, widget::button};

    pub fn mode_tab(theme: &Theme, status: button::Status, active: bool) -> button::Style {
        let palette = theme.extended_palette();
        let mut style = button::secondary(theme, status);

        style.background = Some(Background::Color(match (active, status) {
            (true, _) => palette.background.base.color,
            (false, button::Status::Hovered) => palette.primary.strong.color,
            (false, button::Status::Pressed) => palette.primary.weak.color,
            (false, button::Status::Disabled) => palette.background.weak.color,
            (false, button::Status::Active) => palette.background.strong.color,
        }));

        style.text_color = match (active, status) {
            (_, button::Status::Disabled) => palette.background.weak.text,
            (true, _) => palette.background.base.text,
            (false, button::Status::Pressed) => palette.primary.weak.text,
            (false, button::Status::Hovered) => palette.primary.strong.text,
            (false, button::Status::Active) => palette.background.strong.text,
        };

        style.border = Border {
            width: 0.0,
            radius: Radius::new(0.0).bottom(30.0),
            color: match active {
                true => palette.background.strong.color,
                false => Color {
                    a: 0.45,
                    ..palette.background.strong.color
                },
            },
        };

        style.shadow = iced::Shadow::default();

        style
    }

    pub fn file_list_scrollable(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
        let palette = theme.extended_palette();
        let mut style = scrollable::default(theme, status);
        style.container = container::Style {
            background: Some(palette.background.weak.color.into()),
            border: iced::Border {
                width: 2.0,
                color: palette.background.strong.color,
                ..iced::Border::default()
            },
            ..Default::default()
        };
        style
    }
}

impl DataFileManager {
    fn add_noaa_sources_in_range(&mut self, years: RangeInclusive<i32>) -> Result<(), String> {
        let start_year = *years.start();
        let end_year = *years.end();
        let files = self
            .cached_noaa_index
            .as_ref()
            .into_iter()
            .flat_map(|index| index.iter_in_year_range(years.clone()))
            .map(|file| FileSource::Remote(file.url.clone()))
            .collect::<Vec<_>>();

        match files.is_empty() {
            true => Err(format!(
                "No NOAA files found for years {start_year}..={end_year}."
            )),
            false => {
                self.insert_sources(files);
                Ok(())
            }
        }
    }

    fn can_fetch_noaa_sources(&self) -> bool {
        self.noaa_year_range.parsed().is_some()
    }

    fn is_animating(&self) -> bool {
        self.fetching_noaa_index || self.is_finalizing()
    }

    fn is_finalizing(&self) -> bool {
        self.all_files_complete && self.processing_outcome.is_none()
    }

    fn insert_source(&mut self, source: FileSource) {
        match self.sources.binary_search(&source) {
            Ok(_) => {}
            Err(index) => self.sources.insert(index, source),
        }
    }

    fn insert_sources(&mut self, sources: impl IntoIterator<Item = FileSource>) {
        sources
            .into_iter()
            .for_each(|source| self.insert_source(source));
    }
}

fn source_mode_label<'a>(label: &'a str, active: bool) -> Element<'a, Message> {
    text(label)
        .font(Font {
            weight: match active {
                true => Weight::Bold,
                false => Weight::Normal,
            },
            ..Font::DEFAULT
        })
        .into()
}

fn action_button<'a>(
    label: &'a str,
    tooltip: Option<&'a str>,
    on_press: Option<Message>,
    style: impl Fn(&iced::Theme, button::Status) -> button::Style + 'a,
) -> Element<'a, Message> {
    let button = match on_press {
        Some(message) => button(text(label))
            .padding([8, 12])
            .height(BUTTON_HEIGHT)
            .style(style)
            .on_press(message),
        None => button(text(label))
            .padding([8, 12])
            .height(BUTTON_HEIGHT)
            .style(style),
    };

    match tooltip {
        Some(t) => follow_tooltip(button, text(t)),
        None => button.into(),
    }
}

fn view_source_row<'a>(source: &'a FileSource) -> Element<'a, Message> {
    row([
        container(source_name_view(source))
            .width(Length::Fill)
            .into(),
        action_button(
            "-",
            Some("Remove this source"),
            Some(Message::RemoveSource(source.clone())),
            button::danger,
        ),
    ])
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn source_name_view<'a>(source: &'a FileSource) -> Element<'a, Message> {
    row([
        text(match source {
            FileSource::Local(_) => "LOCAL:",
            FileSource::Remote(_) => "NOAA:",
        })
        .font(Font {
            weight: Weight::Bold,
            ..Font::DEFAULT
        })
        .into(),
        text(display_source_name(source)).into(),
    ])
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn display_source_name(source: &FileSource) -> String {
    match source {
        FileSource::Local(path) => path.display().to_string(),
        FileSource::Remote(url) => url
            .path_segments()
            .and_then(|mut segments| segments.next_back().map(str::to_owned))
            .unwrap_or_else(|| url.as_str().to_owned()),
    }
}

fn current_year() -> i32 {
    chrono::Utc::now().year()
}

fn warning_text_style(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(theme.extended_palette().warning.base.color),
    }
}

fn digits_only(input: String) -> String {
    input.chars().filter(char::is_ascii_digit).collect()
}

fn parse_noaa_year(input: &str) -> Option<i32> {
    input
        .parse::<i32>()
        .ok()
        .filter(|year| (*year >= NOAA_START_YEAR_MIN) && (*year <= current_year()))
}

fn writer_options_view<'a>(options: &'a WriterOptions) -> Element<'a, Message> {
    container(
        scrollable(
            column([
                writer_option_row(
                    "Number of files:",
                    "Number of files",
                    options.number_of_files,
                    |s| {
                        writer_options_parse_uint(&s, |i| WriterOptions {
                            number_of_files: i,
                            ..*options
                        })
                    },
                ),
                writer_option_row(
                    "Max batch size:",
                    "Max batch size",
                    options.max_batch_size,
                    |s| {
                        writer_options_parse_uint(&s, |i| WriterOptions {
                            max_batch_size: i,
                            ..*options
                        })
                    },
                ),
                writer_option_row(
                    "Max in flight:",
                    "Max in flight",
                    options.max_in_flight,
                    |s| {
                        writer_options_parse_uint(&s, |i| WriterOptions {
                            max_in_flight: i,
                            ..*options
                        })
                    },
                ),
                writer_option_row(
                    "Parquet page row count limit:",
                    "Parquet page row count limit",
                    options.parquet_page_row_count_limit,
                    |s| {
                        writer_options_parse_uint(&s, |i| WriterOptions {
                            parquet_page_row_count_limit: i,
                            ..*options
                        })
                    },
                ),
            ])
            .spacing(8),
        )
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn writer_option_row<'a>(
    label: &'static str,
    placeholder: &'static str,
    value: usize,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let value = match value {
        0 => String::new(),
        _ => value.to_string(),
    };

    row([
        text(label)
            .width(Length::Fixed(WRITER_OPTIONS_LABEL_WIDTH))
            .into(),
        text_input(placeholder, &value)
            .on_input(on_input)
            .padding(8)
            .width(Length::Fill)
            .into(),
    ])
    .spacing(12)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn writer_options_parse_uint(s: &str, f: impl Fn(usize) -> WriterOptions) -> Message {
    if s.is_empty() {
        return Message::UpdateWriterOptions(f(0));
    }
    s.parse::<usize>()
        .ok()
        .map(|i| Message::UpdateWriterOptions(f(i)))
        .unwrap_or_default()
}

fn writer_options_unavailable(o: &WriterOptions) -> bool {
    o.number_of_files == 0
        || o.max_batch_size == 0
        || o.max_in_flight == 0
        || o.parquet_page_row_count_limit == 0
}
