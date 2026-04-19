use std::path::Path;
use std::time::UNIX_EPOCH;

use rusqlite::Connection;

use crate::error::Result;

#[path = "sqlite_drawers.rs"]
mod sqlite_drawers;
#[path = "sqlite_kg.rs"]
mod sqlite_kg;
#[path = "sqlite_schema.rs"]
mod sqlite_schema;

#[derive(Clone, Debug, PartialEq)]
pub struct GraphRoomRow {
    pub room: String,
    pub wing: String,
    pub filed_at: Option<String>,
}

pub const CURRENT_SCHEMA_VERSION: i64 = 7;

#[derive(Clone, Debug, PartialEq)]
pub struct IngestedFileState {
    pub content_hash: String,
    pub source_mtime: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawerRecord {
    pub id: String,
    pub wing: String,
    pub room: String,
    pub source_file: String,
    pub source_path: String,
    pub source_hash: String,
    pub source_mtime: Option<f64>,
    pub chunk_index: i32,
    pub added_by: String,
    pub filed_at: String,
    pub ingest_mode: String,
    pub extract_mode: String,
    pub text: String,
}

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn source_mtime(path: &Path) -> Option<f64> {
        let modified = path.metadata().ok()?.modified().ok()?;
        let duration = modified.duration_since(UNIX_EPOCH).ok()?;
        Some(duration.as_secs_f64())
    }
}
