use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MempalaceError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("LanceDB error: {0}")]
    Lance(#[from] lancedb::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow_schema::ArrowError),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("MCP error: {0}")]
    Mcp(String),
}

pub type Result<T> = std::result::Result<T, MempalaceError>;
