//! Error types for Synapse Ultra.

use thiserror::Error;

pub type UltraResult<T> = std::result::Result<T, UltraError>;

#[derive(Debug, Error)]
pub enum UltraError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde json error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("zstd error: {0}")]
    #[cfg(feature = "zstd-compress")]
    Zstd(String),

    #[error("event parse error: {0}")]
    EventParse(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("ducklake error: {0}")]
    DuckLake(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}
