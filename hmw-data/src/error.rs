use hmw_parquet::{datafusion::error::DataFusionError, parquet::errors::ParquetError};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("imma reading error: {0}")]
    ReadError(#[from] imma_files::Error),
    #[error("write error: {0}")]
    WriteError(#[from] ParquetError),
    #[error("error loading lattice")]
    LoadLatticeError,
    #[error("output dir is missing")]
    MissingOutputDir,
    #[error("input dir is missing")]
    MissingInputDir,
    #[error("query error: {0}")]
    DataFusionError(#[from] DataFusionError),
}
