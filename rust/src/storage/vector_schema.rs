//! LanceDB table bootstrap and metadata-column backfill.
//!
//! Search data evolved over time just like the SQLite schema. This module keeps
//! older LanceDB tables readable by adding missing metadata columns lazily when
//! they are first opened.

use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema, SchemaRef};
use lancedb::table::{NewColumnTransform, Table};

use crate::error::Result;

use super::VectorStore;

pub(crate) const TABLE_NAME: &str = "drawers";

impl VectorStore {
    /// Opens the drawers table or creates it if this palace has no vector index yet.
    pub async fn ensure_table(&self, dimension: usize) -> Result<Table> {
        match self.conn.open_table(TABLE_NAME).execute().await {
            Ok(table) => {
                self.ensure_metadata_columns(&table).await?;
                Ok(table)
            }
            Err(_) => {
                let schema = schema(dimension);
                let table = self
                    .conn
                    .create_empty_table(TABLE_NAME, schema)
                    .execute()
                    .await?;
                self.ensure_metadata_columns(&table).await?;
                Ok(table)
            }
        }
    }

    async fn ensure_metadata_columns(&self, table: &Table) -> Result<()> {
        let table_schema = table.schema().await?;
        let mut transforms = Vec::new();

        if table_schema.field_with_name("source_file").is_err() {
            transforms.push(("source_file".into(), "source_path".into()));
        }
        if table_schema.field_with_name("source_mtime").is_err() {
            transforms.push(("source_mtime".into(), "CAST(NULL AS DOUBLE)".into()));
        }
        if table_schema.field_with_name("added_by").is_err() {
            transforms.push(("added_by".into(), "'mempalace'".into()));
        }
        if table_schema.field_with_name("filed_at").is_err() {
            transforms.push(("filed_at".into(), "CAST(NULL AS STRING)".into()));
        }
        if table_schema.field_with_name("ingest_mode").is_err() {
            transforms.push(("ingest_mode".into(), "'projects'".into()));
        }
        if table_schema.field_with_name("extract_mode").is_err() {
            transforms.push(("extract_mode".into(), "'exchange'".into()));
        }

        if !transforms.is_empty() {
            // Old vector tables are backfilled in place so search and repair
            // code can rely on the newer metadata contract afterwards.
            table
                .add_columns(NewColumnTransform::SqlExpressions(transforms), None)
                .await?;
        }

        Ok(())
    }
}

/// Returns the canonical Arrow schema for the LanceDB drawers table.
pub(crate) fn schema(dimension: usize) -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("wing", DataType::Utf8, false),
        Field::new("room", DataType::Utf8, false),
        Field::new("source_file", DataType::Utf8, false),
        Field::new("source_path", DataType::Utf8, false),
        Field::new("source_mtime", DataType::Float64, true),
        Field::new("chunk_index", DataType::Int32, false),
        Field::new("added_by", DataType::Utf8, true),
        Field::new("filed_at", DataType::Utf8, true),
        Field::new("ingest_mode", DataType::Utf8, true),
        Field::new("extract_mode", DataType::Utf8, true),
        Field::new("text", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dimension as i32,
            ),
            true,
        ),
    ]))
}
