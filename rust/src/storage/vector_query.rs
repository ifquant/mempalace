//! LanceDB query and mutation helpers.
//!
//! These methods manage the vector-side index only. Higher-level runtimes are
//! responsible for keeping SQLite and LanceDB in sync when one store succeeds
//! and the other fails.

use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};

use crate::error::{MempalaceError, Result};
use crate::model::{DrawerInput, SearchHit};

use super::{VectorDrawer, VectorStore, record_batch, vector_drawers_from_batch};

impl VectorStore {
    /// Replaces every vector row for one source path with the new embedding batch.
    pub async fn replace_source(
        &self,
        drawers: &[DrawerInput],
        embeddings: &[Vec<f32>],
    ) -> Result<()> {
        if drawers.is_empty() {
            return Ok(());
        }
        let dimension = embeddings
            .first()
            .map(Vec::len)
            .ok_or_else(|| MempalaceError::InvalidArgument("missing embeddings".to_string()))?;
        let table = self.ensure_table(dimension).await?;
        if let Some(source_path) = drawers.first().map(|drawer| drawer.source_path.clone()) {
            let escaped = source_path.replace('\'', "''");
            // Source replacement is expressed as delete-then-add because LanceDB
            // does not expose the SQLite-style transaction used by the canonical store.
            table.delete(&format!("source_path = '{escaped}'")).await?;
        }
        let batch = record_batch(drawers, embeddings, dimension)?;
        table.add(batch).execute().await?;
        Ok(())
    }

    /// Runs a nearest-neighbor search with optional wing/room filters.
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

    /// Appends new drawer vectors without deleting prior rows.
    pub async fn add_drawers(
        &self,
        drawers: &[DrawerInput],
        embeddings: &[Vec<f32>],
    ) -> Result<()> {
        if drawers.is_empty() {
            return Ok(());
        }
        let dimension = embeddings
            .first()
            .map(Vec::len)
            .ok_or_else(|| MempalaceError::InvalidArgument("missing embeddings".to_string()))?;
        let table = self.ensure_table(dimension).await?;
        let batch = record_batch(drawers, embeddings, dimension)?;
        table.add(batch).execute().await?;
        Ok(())
    }

    /// Returns whether a vector row exists for a drawer ID.
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

    /// Deletes one vector row by drawer ID.
    pub async fn delete_drawer(&self, dimension: usize, drawer_id: &str) -> Result<()> {
        let table = self.ensure_table(dimension).await?;
        let escaped = drawer_id.replace('\'', "''");
        table.delete(&format!("id = '{escaped}'")).await?;
        Ok(())
    }

    /// Deletes multiple vector rows and reports how many delete attempts were issued.
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

    /// Clears the entire drawers table for a given embedding dimension.
    pub async fn clear_table(&self, dimension: usize) -> Result<()> {
        let table = self.ensure_table(dimension).await?;
        table.delete("id IS NOT NULL").await?;
        Ok(())
    }

    /// Lists stored vector rows for maintenance and repair scans.
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

pub(crate) fn search_hits_from_batch(batch: &arrow_array::RecordBatch) -> Vec<SearchHit> {
    let ids = batch
        .column_by_name("id")
        .expect("id")
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .expect("id string");
    let wings = batch
        .column_by_name("wing")
        .expect("wing")
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .expect("wing string");
    let rooms = batch
        .column_by_name("room")
        .expect("room")
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .expect("room string");
    let source_files = batch
        .column_by_name("source_file")
        .and_then(|col| col.as_any().downcast_ref::<arrow_array::StringArray>());
    let source_paths = batch
        .column_by_name("source_path")
        .expect("source_path")
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .expect("source_path string");
    let source_mtimes = batch
        .column_by_name("source_mtime")
        .and_then(|col| col.as_any().downcast_ref::<arrow_array::Float64Array>());
    let chunk_indices = batch
        .column_by_name("chunk_index")
        .expect("chunk_index")
        .as_any()
        .downcast_ref::<arrow_array::Int32Array>()
        .expect("chunk_index int");
    let added_bys = batch
        .column_by_name("added_by")
        .and_then(|col| col.as_any().downcast_ref::<arrow_array::StringArray>());
    let filed_ats = batch
        .column_by_name("filed_at")
        .and_then(|col| col.as_any().downcast_ref::<arrow_array::StringArray>());
    let texts = batch
        .column_by_name("text")
        .expect("text")
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .expect("text string");
    let score_f32 = batch
        .column_by_name("_distance")
        .and_then(|col| col.as_any().downcast_ref::<arrow_array::Float32Array>());

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

pub(crate) fn filter_sql(wing: Option<&str>, room: Option<&str>) -> Option<String> {
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

pub(crate) fn filter_source_sql(
    wing: Option<&str>,
    source_pattern: Option<&str>,
) -> Option<String> {
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

fn derive_source_file(source_path: &str) -> String {
    std::path::Path::new(source_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| source_path.to_string())
}

fn nullable_string(values: Option<&arrow_array::StringArray>, row: usize) -> Option<String> {
    use arrow_array::Array;
    values.and_then(|values| {
        if values.is_null(row) {
            None
        } else {
            Some(values.value(row).to_string())
        }
    })
}

fn nullable_f64(values: Option<&arrow_array::Float64Array>, row: usize) -> Option<f64> {
    use arrow_array::Array;
    values.and_then(|values| {
        if values.is_null(row) {
            None
        } else {
            Some(values.value(row))
        }
    })
}
