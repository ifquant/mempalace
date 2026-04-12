use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct DrawerInput {
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
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub id: String,
    pub text: String,
    pub wing: String,
    pub room: String,
    pub source_file: String,
    pub source_path: String,
    pub source_mtime: Option<f64>,
    pub chunk_index: i32,
    pub added_by: Option<String>,
    pub filed_at: Option<String>,
    pub similarity: Option<f64>,
    pub score: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchFilters {
    pub wing: Option<String>,
    pub room: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchResults {
    pub query: String,
    pub filters: SearchFilters,
    pub results: Vec<SearchHit>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub kind: String,
    pub total_drawers: usize,
    pub wings: BTreeMap<String, usize>,
    pub rooms: BTreeMap<String, usize>,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub schema_version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rooms {
    pub wing: String,
    pub rooms: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Taxonomy {
    pub taxonomy: BTreeMap<String, BTreeMap<String, usize>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KgTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MineSummary {
    pub kind: String,
    pub mode: String,
    pub extract: String,
    pub agent: String,
    pub wing: String,
    pub configured_rooms: Vec<String>,
    pub project_path: String,
    pub palace_path: String,
    pub version: String,
    pub dry_run: bool,
    pub filters: SearchFilters,
    pub respect_gitignore: bool,
    pub include_ignored: Vec<String>,
    pub files_planned: usize,
    pub files_seen: usize,
    pub files_mined: usize,
    pub drawers_added: usize,
    pub files_skipped_unchanged: usize,
    pub room_counts: BTreeMap<String, usize>,
    pub next_hint: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MineRequest {
    pub wing: Option<String>,
    pub mode: String,
    pub agent: String,
    pub limit: usize,
    pub dry_run: bool,
    pub respect_gitignore: bool,
    pub include_ignored: Vec<String>,
    pub extract: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MineProgressEvent {
    DryRun {
        file_name: String,
        room: String,
        drawers: usize,
    },
    Filed {
        index: usize,
        total: usize,
        file_name: String,
        drawers: usize,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InitSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub schema_version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MigrateSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub version: String,
    pub schema_version_before: Option<i64>,
    pub schema_version_after: i64,
    pub changed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RepairSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub sqlite_exists: bool,
    pub lance_exists: bool,
    pub schema_version: Option<i64>,
    pub sqlite_drawer_count: Option<usize>,
    pub embedding_provider: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dimension: Option<usize>,
    pub vector_accessible: bool,
    pub ok: bool,
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DoctorSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub provider: String,
    pub model: String,
    pub dimension: usize,
    pub cache_dir: Option<String>,
    pub model_cache_dir: Option<String>,
    pub model_cache_present: bool,
    pub expected_model_file: Option<String>,
    pub expected_model_file_present: bool,
    pub hf_endpoint: Option<String>,
    pub ort_dylib_path: Option<String>,
    pub warmup_attempted: bool,
    pub warmup_ok: bool,
    pub warmup_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PrepareEmbeddingSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub provider: String,
    pub model: String,
    pub attempts: usize,
    pub success: bool,
    pub last_error: Option<String>,
    pub doctor: DoctorSummary,
}
