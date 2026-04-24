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

pub struct VectorStore {
    conn: Connection,
}

impl VectorStore {
    pub async fn connect(path: &std::path::Path) -> Result<Self> {
        let conn = lancedb::connect(path.to_string_lossy().as_ref())
            .execute()
            .await?;
        Ok(Self { conn })
    }
}
