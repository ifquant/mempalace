//! LanceDB-backed vector search store for palace drawers.
//!
//! LanceDB is a secondary index layered on top of canonical SQLite metadata.
//! It stores embeddings plus enough drawer metadata to support search and
//! maintenance scans without becoming the source of truth.

use lancedb::Connection;

use crate::error::Result;

#[path = "vector_batch.rs"]
mod vector_batch;
#[path = "vector_query.rs"]
mod vector_query;
#[path = "vector_schema.rs"]
mod vector_schema;

pub(crate) use vector_batch::{record_batch, vector_drawers_from_batch};
pub(crate) use vector_schema::schema;

/// Drawer row materialized from LanceDB batches during search and maintenance.
#[derive(Clone, Debug)]
pub struct VectorDrawer {
    pub id: String,
    pub wing: String,
    pub room: String,
    pub source_file: String,
    pub source_path: String,
    pub text: String,
    pub vector: Vec<f32>,
}

/// Thin wrapper around the LanceDB connection used by the palace runtime.
pub struct VectorStore {
    conn: Connection,
}

impl VectorStore {
    /// Opens the LanceDB directory for the current palace.
    pub async fn connect(path: &std::path::Path) -> Result<Self> {
        let conn = lancedb::connect(path.to_string_lossy().as_ref())
            .execute()
            .await?;
        Ok(Self { conn })
    }
}
