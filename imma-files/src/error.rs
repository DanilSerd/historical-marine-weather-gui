use crate::source::FileSource;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("http error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("remote download error: {0}")]
    RemoteDownloadError(String),
    #[error("remote file index error: {0}")]
    RemoteFileIndexError(String),
    #[error("remote listing parse error: {0}")]
    RemoteListingParseError(String),
    #[error("join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("file extention is not supported")]
    UnsoportedFileExtention,
    #[error("send error: {0}")]
    ChannelSendError(&'static str),
    #[error("imma iter error: {0}")]
    IMMAIterError(#[from] imma_parser::iter::Error),
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::ChannelSendError("failed mpsc channel send")
    }
}

impl<T> From<async_channel::SendError<T>> for Error {
    fn from(_: async_channel::SendError<T>) -> Self {
        Self::ChannelSendError("failed async channel send")
    }
}

#[derive(thiserror::Error, Debug)]
#[error("file error on {file}: {error}")]
pub struct FileError {
    /// The file source that failed.
    pub file: FileSource,
    /// The error that occurred while processing the file source.
    pub error: Error,
}
