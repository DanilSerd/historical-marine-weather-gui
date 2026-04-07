use std::path::PathBuf;

use iced::Task;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[derive(Debug)]
pub struct AppPersistentStateManager {
    config: AppPersistentConfig,
    tx: tokio::sync::mpsc::UnboundedSender<AppPersistentConfig>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AppPersistentConfig {
    pub data_dir: Option<PathBuf>,
    pub dark_mode: Option<bool>,
}

#[derive(Error, Debug)]
pub enum PersistentStateError {
    #[error("config dir not found")]
    DirNotFound,
    #[error("config file error")]
    File(#[from] tokio::io::Error),
    #[error("config send error")]
    Send(#[from] tokio::sync::mpsc::error::SendError<AppPersistentConfig>),
}

impl AppPersistentStateManager {
    pub async fn open()
    -> Result<(Self, Task<Result<(), PersistentStateError>>), PersistentStateError> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let dirs = directories::ProjectDirs::from("", "", "historical-marine-weather")
            .ok_or(PersistentStateError::DirNotFound)?;
        tokio::fs::create_dir_all(dirs.config_dir()).await?;
        let config_file = dirs.config_dir().join("config.json");
        let mut config_file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(false)
            .truncate(false)
            .read(true)
            .open(config_file)
            .await?;
        let mut contents = String::new();
        config_file.read_to_string(&mut contents).await?;
        let config = serde_json::from_str(&contents).unwrap_or_default();
        let task = Task::future(Self::save(rx, config_file));

        Ok((Self { config, tx }, task))
    }

    pub fn config(&self) -> &AppPersistentConfig {
        &self.config
    }

    pub fn is_dark_mode(&self) -> bool {
        match self.config.dark_mode {
            Some(true) => true,
            Some(false) | None => false,
        }
    }

    pub fn update_data_dir(&mut self, data_dir: PathBuf) -> Result<(), PersistentStateError> {
        self.config.data_dir = Some(data_dir);
        self.tx.send(self.config.clone())?;
        Ok(())
    }

    pub fn update_dark_mode(&mut self, dark_mode: bool) -> Result<(), PersistentStateError> {
        self.config.dark_mode = Some(dark_mode);
        self.tx.send(self.config.clone())?;
        Ok(())
    }

    async fn save(
        mut rx: tokio::sync::mpsc::UnboundedReceiver<AppPersistentConfig>,
        mut config_file: tokio::fs::File,
    ) -> Result<(), PersistentStateError> {
        while let Some(config) = rx.recv().await {
            let serialized = serde_json::to_string(&config).expect("can serialize config");
            config_file.set_len(0).await?;
            config_file.rewind().await?;
            config_file.write_all(serialized.as_bytes()).await?;
            config_file.flush().await?;
            config_file.sync_all().await?;
        }
        Ok(())
    }
}
