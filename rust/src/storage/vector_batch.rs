//! Arrow batch conversions used by the LanceDB storage layer.

use std::sync::Arc;

use arrow_array::types::Float32Type;
use arrow_array::{FixedSizeListArray, Float64Array, Int32Array, RecordBatch, StringArray};

use crate::error::Result;
use crate::model::DrawerInput;

use super::{VectorDrawer, schema};

/// Builds the Arrow batch written into LanceDB for a drawer slice.
pub(crate) fn record_batch(
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

/// Converts a LanceDB query batch back into vector-drawer rows.
pub(crate) fn vector_drawers_from_batch(batch: &RecordBatch) -> Vec<VectorDrawer> {
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
        .downcast_ref::<arrow_array::Float32Array>()
        .expect("vector float values");
    (0..values.len()).map(|index| values.value(index)).collect()
}
