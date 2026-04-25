//! SQLite-backed canonical metadata store for the palace rewrite.
//!
//! SQLite owns schema versioning, drawer metadata, KG rows, and maintenance
//! bookkeeping. LanceDB is a secondary index for semantic search, not the
//! source of truth.

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

/// Minimal room-edge row used to build the palace traversal graph.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphRoomRow {
    pub room: String,
    pub wing: String,
    pub filed_at: Option<String>,
}

/// Current on-disk SQLite schema version supported by the Rust rewrite.
pub const CURRENT_SCHEMA_VERSION: i64 = 9;

/// Cached ingest state for deciding whether a source file needs re-mining.
#[derive(Clone, Debug, PartialEq)]
pub struct IngestedFileState {
    pub content_hash: String,
    pub source_mtime: Option<f64>,
}

/// Canonical drawer row loaded from SQLite.
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
    pub importance: Option<f64>,
    pub text: String,
}

/// Thin wrapper around the palace SQLite connection.
pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    /// Opens the palace SQLite database file.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    /// Returns a source file mtime in seconds since the Unix epoch.
    pub fn source_mtime(path: &Path) -> Option<f64> {
        let modified = path.metadata().ok()?.modified().ok()?;
        let duration = modified.duration_since(UNIX_EPOCH).ok()?;
        Some(duration.as_secs_f64())
    }
}
