use std::{collections::HashMap, path::Path};

use hmw_data::DataVersion;
use iced::{Task, task::Handle};
use rfd::FileHandle;
use serde::{Deserialize, Serialize};

use crate::{
    loader::{Loader, LoaderStats},
    types::{WeatherSummary, WeatherSummaryId, WeatherSummaryKindEnum, WeatherSummaryParams},
};

pub struct WeatherSummaryCollection {
    summaries: HashMap<WeatherSummaryId, WeatherSummary>,
    load_abort_handles: HashMap<WeatherSummaryId, Handle>,
    loader: Loader,
    saving: Saving,
}

impl WeatherSummaryCollection {
    pub fn new(loader: Loader) -> Self {
        Self {
            summaries: HashMap::new(),
            load_abort_handles: HashMap::new(),
            loader,
            saving: Saving::default(),
        }
    }

    pub fn start_refresh(&mut self, loader: Loader) {
        // TODO: SOmething strange is going on. If i pick bad loader dir, fetch data and then switch to good loader dir,
        // the data is still not loaded. So the error for a particular summury is maintained, or the loader is somehow is persisting.
        self.loader = loader;
        self.load_abort_handles.clear();
        self.summaries
            .values_mut()
            .for_each(|s| s.invalidate_data());
    }

    pub fn open(file: FileHandle, loader: Loader) -> Task<Result<Self, &'static str>> {
        let open_future = async move {
            let serialized = file.read().await;
            let file_format: FileFormat =
                serde_json::from_slice(&serialized).map_err(|_| "Failed to deserialize")?;

            if file_format.version != loader.data_version() {
                return Err("Data version mismatch");
            }

            Ok(Self {
                summaries: file_format.summaries,
                load_abort_handles: HashMap::new(),
                loader,
                saving: Saving::new(Some(file), SavingStatus::Saved),
            })
        };

        Task::future(open_future)
    }

    pub fn finish_open(&mut self) -> Task<WeatherSummary> {
        let ids = self.summaries.keys().copied().collect::<Vec<_>>();
        let loading_tasks = ids.iter().map(|id| self.load_data(id));

        Task::batch(loading_tasks)
    }

    pub fn get(&self, id: &WeatherSummaryId) -> Option<&WeatherSummary> {
        self.summaries.get(id)
    }

    pub fn add(
        &mut self,
        summary: WeatherSummaryParams,
        kind: WeatherSummaryKindEnum,
    ) -> Task<WeatherSummary> {
        let id = summary.header.id;
        self.summaries
            .insert(id, WeatherSummary::new(summary, kind));
        self.saving.mark_unsaved();
        self.load_data(&id)
    }

    fn load_data(&mut self, id: &WeatherSummaryId) -> Task<WeatherSummary> {
        let summary = match self.get(id) {
            Some(summary) if !summary.data_avaialble().unwrap_or_default() => summary,
            _ => return Task::none(),
        };

        let summary_clone = summary.clone();
        let id = summary.params().header.id;
        let loader = self.loader.clone();

        let future = summary_clone.populate_data(loader);
        let (task, handle) = Task::future(future).abortable();
        self.load_abort_handles.insert(id, handle.abort_on_drop());
        task
    }

    pub fn remove(&mut self, id: &WeatherSummaryId) {
        self.load_abort_handles.remove(id);
        self.summaries.remove(id);
        self.saving.mark_unsaved();
    }

    pub fn finish_load(&mut self, summary: WeatherSummary) {
        let id = summary.params().header.id;
        if let Some(s) = self.summaries.get_mut(&id) {
            #[cfg(debug_assertions)]
            if let Err(e) = summary.data_avaialble() {
                dbg!(format!("Error loading data for {id}: {e}"));
            }
            *s = summary;
        }
        self.load_abort_handles.remove(&id);
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&WeatherSummaryId, &WeatherSummary)> {
        self.summaries.iter()
    }

    /// Returns a tuple of (savable, savable_as)
    pub fn savable(&self) -> (bool, bool) {
        (self.saving.savable(), self.saving.savable_as())
    }

    pub fn save(&mut self) -> Result<Task<std::io::Result<FileHandle>>, &'static str> {
        let file = match self.saving.start_saving() {
            Some(file) => file,
            None => {
                self.saving.mark_unsaved();
                return Err("Not savable");
            }
        };

        let format = FileFormatRef {
            version: self.loader.data_version(),
            summaries: &self.summaries,
        };

        let serialized =
            serde_json::to_vec(&format).map_err(|_| "Failed to serialize collection")?;

        let future = async move { file.write(&serialized).await.map(|_| file) };

        Ok(Task::future(future))
    }

    pub fn change_file(&mut self, file: FileHandle) -> Result<(), &'static str> {
        if !self.saving.savable_as() {
            return Err("Not savable as");
        }

        self.saving = Saving::new(Some(file), SavingStatus::Unsaved);
        Ok(())
    }

    pub fn finish_save(&mut self, file: FileHandle) {
        self.saving.finish_saving(file);
    }

    pub fn save_details(&self) -> (Option<&Path>, SavingStatus) {
        let file_path = self.saving.file.as_ref().map(|fh| fh.path());
        (file_path, self.saving.status)
    }

    pub fn stats(&self) -> &LoaderStats {
        self.loader.stats()
    }
}

#[derive(Debug, Clone, Default)]
struct Saving {
    file: Option<FileHandle>,
    status: SavingStatus,
}

impl Saving {
    fn new(file: Option<FileHandle>, status: SavingStatus) -> Self {
        Self { file, status }
    }

    fn mark_unsaved(&mut self) {
        self.status = SavingStatus::Unsaved;
    }

    fn start_saving(&mut self) -> Option<FileHandle> {
        let fh = match (&self.status, self.file.take()) {
            (SavingStatus::Unsaved, fh @ Some(_)) => fh,
            _ => None,
        };
        self.status = SavingStatus::Saving;
        fh
    }

    fn finish_saving(&mut self, file: FileHandle) {
        self.file = Some(file);
        if self.status == SavingStatus::Saving {
            self.status = SavingStatus::Saved;
        }
    }

    fn savable(&self) -> bool {
        if self.file.is_some() && self.status == SavingStatus::Unsaved {
            return true;
        }
        false
    }

    fn savable_as(&self) -> bool {
        if self.status != SavingStatus::Saving {
            return true;
        }
        false
    }
}

#[derive(Debug, Clone, Default, PartialEq, Copy)]
pub enum SavingStatus {
    Saving,
    Saved,
    #[default]
    Unsaved,
}

impl std::fmt::Display for SavingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SavingStatus::Saving => write!(f, "Saving"),
            SavingStatus::Saved => write!(f, "Saved"),
            SavingStatus::Unsaved => write!(f, "Unsaved"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct FileFormat {
    version: DataVersion,
    summaries: HashMap<WeatherSummaryId, WeatherSummary>,
}

#[derive(Debug, Serialize)]
struct FileFormatRef<'a> {
    version: DataVersion,
    summaries: &'a HashMap<WeatherSummaryId, WeatherSummary>,
}
