mod reader;
mod traits;
mod writer;

pub use reader::*;
use serde::{Deserialize, Serialize};
pub use traits::*;
pub use writer::*;

pub use arrow;
pub use arrow_convert;
pub use datafusion;
pub use parquet;

#[derive(
    Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display,
)]
pub enum DataVersion {
    #[default]
    V1,
}

pub fn data_file_prefix(version: DataVersion) -> String {
    format!("hmwdata_{}", version)
}
