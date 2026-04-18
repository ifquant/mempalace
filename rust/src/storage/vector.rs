use std::path::Path;
use std::sync::Arc;

use arrow_array::types::Float32Type;
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Float64Array, Int32Array, RecordBatch, StringArray,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::NewColumnTransform;
use lancedb::{Connection, Table, connect};

use crate::error::Result;
use crate::model::{DrawerInput, SearchHit};

const TABLE_NAME: &str = "drawers";

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
    pub async fn connect(path: &Path) -> Result<Self> {
        let conn = connect(path.to_string_lossy().as_ref()).execute().await?;
        Ok(Self { conn })
    }

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

    pub async fn replace_source(
        &self,
        drawers: &[DrawerInput],
        embeddings: &[Vec<f32>],
    ) -> Result<()> {
        if drawers.is_empty() {
            return Ok(());
        }
        let dimension = embeddings.first().map(Vec::len).ok_or_else(|| {
            crate::error::MempalaceError::InvalidArgument("missing embeddings".to_string())
        })?;
        let table = self.ensure_table(dimension).await?;
        if let Some(source_path) = drawers.first().map(|drawer| drawer.source_path.clone()) {
            let escaped = source_path.replace('\'', "''");
            table
                .delete(&format!("source_path = '{}'", escaped))
                .await?;
        }
        let batch = record_batch(drawers, embeddings, dimension)?;
        table.add(batch).execute().await?;
        Ok(())
    }

    pub async fn search(
        &self,
        embedding: &[f32],
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let table = self.ensure_table(embedding.len()).await?;
        let mut query = table.query().limit(limit);
        if let Some(filter) = filter_sql(wing, room) {
            query = query.only_if(filter);
        }

        let batches = query
            .nearest_to(embedding)?
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        let mut hits = Vec::new();
        for batch in batches {
            hits.extend(search_hits_from_batch(&batch));
        }
        Ok(hits)
    }

    pub async fn add_drawers(
        &self,
        drawers: &[DrawerInput],
        embeddings: &[Vec<f32>],
    ) -> Result<()> {
        if drawers.is_empty() {
            return Ok(());
        }
        let dimension = embeddings.first().map(Vec::len).ok_or_else(|| {
            crate::error::MempalaceError::InvalidArgument("missing embeddings".to_string())
        })?;
        let table = self.ensure_table(dimension).await?;
        let batch = record_batch(drawers, embeddings, dimension)?;
        table.add(batch).execute().await?;
        Ok(())
    }

    pub async fn drawer_exists(&self, dimension: usize, drawer_id: &str) -> Result<bool> {
        let table = self.ensure_table(dimension).await?;
        let escaped = drawer_id.replace('\'', "''");
        let batches = table
            .query()
            .only_if(format!("id = '{escaped}'"))
            .limit(1)
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        Ok(batches.iter().any(|batch| batch.num_rows() > 0))
    }

    pub async fn delete_drawer(&self, dimension: usize, drawer_id: &str) -> Result<()> {
        let table = self.ensure_table(dimension).await?;
        let escaped = drawer_id.replace('\'', "''");
        table.delete(&format!("id = '{escaped}'")).await?;
        Ok(())
    }

    pub async fn delete_drawers(&self, dimension: usize, drawer_ids: &[String]) -> Result<usize> {
        if drawer_ids.is_empty() {
            return Ok(0);
        }
        let table = self.ensure_table(dimension).await?;
        let mut deleted = 0usize;
        for drawer_id in drawer_ids {
            let escaped = drawer_id.replace('\'', "''");
            table.delete(&format!("id = '{escaped}'")).await?;
            deleted += 1;
        }
        Ok(deleted)
    }

    pub async fn clear_table(&self, dimension: usize) -> Result<()> {
        let table = self.ensure_table(dimension).await?;
        table.delete("id IS NOT NULL").await?;
        Ok(())
    }

    pub async fn list_drawers(
        &self,
        dimension: usize,
        wing: Option<&str>,
        source_pattern: Option<&str>,
    ) -> Result<Vec<VectorDrawer>> {
        let table = self.ensure_table(dimension).await?;
        let mut query = table.query();
        if let Some(filter) = filter_source_sql(wing, source_pattern) {
            query = query.only_if(filter);
        }
        let batches = query.execute().await?.try_collect::<Vec<_>>().await?;
        let mut drawers = Vec::new();
        for batch in batches {
            drawers.extend(vector_drawers_from_batch(&batch));
        }
        Ok(drawers)
    }
}

impl VectorStore {
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
            table
                .add_columns(NewColumnTransform::SqlExpressions(transforms), None)
                .await?;
        }

        Ok(())
    }
}

fn schema(dimension: usize) -> SchemaRef {
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

fn record_batch(
    drawers: &[DrawerInput],
    embeddings: &[Vec<f32>],
    dimension: usize,
) -> Result<RecordBatch> {
    let schema = schema(dimension);
    let ids = StringArray::from_iter_values(drawers.iter().map(|d| d.id.as_str()));
    let wings = StringArray::from_iter_values(drawers.iter().map(|d| d.wing.as_str()));
    let rooms = StringArray::from_iter_values(drawers.iter().map(|d| d.room.as_str()));
    let source_files =
        StringArray::from_iter_values(drawers.iter().map(|d| d.source_file.as_str()));
    let source_paths =
        StringArray::from_iter_values(drawers.iter().map(|d| d.source_path.as_str()));
    let source_mtimes = Float64Array::from_iter(drawers.iter().map(|d| d.source_mtime));
    let chunk_indices = Int32Array::from_iter_values(drawers.iter().map(|d| d.chunk_index));
    let added_bys = StringArray::from_iter(drawers.iter().map(|d| Some(d.added_by.as_str())));
    let filed_ats = StringArray::from_iter(drawers.iter().map(|d| Some(d.filed_at.as_str())));
    let ingest_modes = StringArray::from_iter(drawers.iter().map(|d| Some(d.ingest_mode.as_str())));
    let extract_modes =
        StringArray::from_iter(drawers.iter().map(|d| Some(d.extract_mode.as_str())));
    let texts = StringArray::from_iter_values(drawers.iter().map(|d| d.text.as_str()));
    let vectors = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        embeddings
            .iter()
            .map(|embedding| Some(embedding.iter().copied().map(Some).collect::<Vec<_>>())),
        dimension as i32,
    );

    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(wings),
            Arc::new(rooms),
            Arc::new(source_files),
            Arc::new(source_paths),
            Arc::new(source_mtimes),
            Arc::new(chunk_indices),
            Arc::new(added_bys),
            Arc::new(filed_ats),
            Arc::new(ingest_modes),
            Arc::new(extract_modes),
            Arc::new(texts),
            Arc::new(vectors),
        ],
    )?)
}

fn search_hits_from_batch(batch: &RecordBatch) -> Vec<SearchHit> {
    let ids = batch
        .column_by_name("id")
        .expect("id")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("id string");
    let wings = batch
        .column_by_name("wing")
        .expect("wing")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("wing string");
    let rooms = batch
        .column_by_name("room")
        .expect("room")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("room string");
    let source_files = batch
        .column_by_name("source_file")
        .and_then(|col| col.as_any().downcast_ref::<StringArray>());
    let source_paths = batch
        .column_by_name("source_path")
        .expect("source_path")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("source_path string");
    let source_mtimes = batch
        .column_by_name("source_mtime")
        .and_then(|col| col.as_any().downcast_ref::<Float64Array>());
    let chunk_indices = batch
        .column_by_name("chunk_index")
        .expect("chunk_index")
        .as_any()
        .downcast_ref::<Int32Array>()
        .expect("chunk_index int");
    let added_bys = batch
        .column_by_name("added_by")
        .and_then(|col| col.as_any().downcast_ref::<StringArray>());
    let filed_ats = batch
        .column_by_name("filed_at")
        .and_then(|col| col.as_any().downcast_ref::<StringArray>());
    let texts = batch
        .column_by_name("text")
        .expect("text")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("text string");
    let score_f32 = batch
        .column_by_name("_distance")
        .and_then(|col| col.as_any().downcast_ref::<Float32Array>());

    let mut hits = Vec::with_capacity(batch.num_rows());
    for row in 0..batch.num_rows() {
        let source_path = source_paths.value(row).to_string();
        let score = score_f32.map(|scores| scores.value(row) as f64);
        let source_file = source_files
            .map(|files| files.value(row).to_string())
            .unwrap_or_else(|| derive_source_file(&source_path));
        hits.push(SearchHit {
            id: ids.value(row).to_string(),
            text: texts.value(row).to_string(),
            wing: wings.value(row).to_string(),
            room: rooms.value(row).to_string(),
            source_file,
            source_path,
            source_mtime: nullable_f64(source_mtimes, row),
            chunk_index: chunk_indices.value(row),
            added_by: nullable_string(added_bys, row),
            filed_at: nullable_string(filed_ats, row),
            similarity: score.map(|distance| (1.0 - distance).clamp(0.0, 1.0)),
            score,
        });
    }
    hits
}

fn vector_drawers_from_batch(batch: &RecordBatch) -> Vec<VectorDrawer> {
    let ids = batch
        .column_by_name("id")
        .expect("id")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("id string");
    let wings = batch
        .column_by_name("wing")
        .expect("wing")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("wing string");
    let rooms = batch
        .column_by_name("room")
        .expect("room")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("room string");
    let source_files = batch
        .column_by_name("source_file")
        .expect("source_file")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("source_file string");
    let source_paths = batch
        .column_by_name("source_path")
        .expect("source_path")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("source_path string");
    let texts = batch
        .column_by_name("text")
        .expect("text")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("text string");
    let vectors = batch
        .column_by_name("vector")
        .expect("vector")
        .as_any()
        .downcast_ref::<FixedSizeListArray>()
        .expect("vector list");

    let mut rows = Vec::with_capacity(batch.num_rows());
    for row in 0..batch.num_rows() {
        rows.push(VectorDrawer {
            id: ids.value(row).to_string(),
            wing: wings.value(row).to_string(),
            room: rooms.value(row).to_string(),
            source_file: source_files.value(row).to_string(),
            source_path: source_paths.value(row).to_string(),
            text: texts.value(row).to_string(),
            vector: vector_from_row(vectors, row),
        });
    }
    rows
}

fn vector_from_row(vectors: &FixedSizeListArray, row: usize) -> Vec<f32> {
    let values = vectors.value(row);
    let values = values
        .as_any()
        .downcast_ref::<Float32Array>()
        .expect("vector float values");
    (0..values.len()).map(|index| values.value(index)).collect()
}

fn derive_source_file(source_path: &str) -> String {
    std::path::Path::new(source_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| source_path.to_string())
}

fn nullable_string(values: Option<&StringArray>, row: usize) -> Option<String> {
    values.and_then(|values| {
        if values.is_null(row) {
            None
        } else {
            Some(values.value(row).to_string())
        }
    })
}

fn nullable_f64(values: Option<&Float64Array>, row: usize) -> Option<f64> {
    values.and_then(|values| {
        if values.is_null(row) {
            None
        } else {
            Some(values.value(row))
        }
    })
}

fn filter_sql(wing: Option<&str>, room: Option<&str>) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(wing) = wing {
        parts.push(format!("wing = '{}'", wing.replace('\'', "''")));
    }
    if let Some(room) = room {
        parts.push(format!("room = '{}'", room.replace('\'', "''")));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" AND "))
    }
}

fn filter_source_sql(wing: Option<&str>, source_pattern: Option<&str>) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(wing) = wing {
        parts.push(format!("wing = '{}'", wing.replace('\'', "''")));
    }
    if let Some(pattern) = source_pattern {
        parts.push(format!(
            "source_file LIKE '%{}%'",
            pattern.replace('\'', "''")
        ));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" AND "))
    }
}
