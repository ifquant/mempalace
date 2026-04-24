use serde::{Deserialize, Serialize};

use super::palace::{CompressedDrawer, SearchHit};

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
pub struct RepairScanSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub wing: Option<String>,
    pub sqlite_drawers: usize,
    pub vector_drawers: usize,
    pub missing_from_vector: Vec<String>,
    pub orphaned_in_vector: Vec<String>,
    pub corrupt_ids_path: String,
    pub prune_candidates: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RepairPruneSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub corrupt_ids_path: String,
    pub queued: usize,
    pub confirm: bool,
    pub deleted_from_vector: usize,
    pub deleted_from_sqlite: usize,
    pub failed: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RepairRebuildSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub drawers_found: usize,
    pub rebuilt: usize,
    pub backup_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DedupSourceResult {
    pub source_file: String,
    pub before: usize,
    pub kept: usize,
    pub deleted: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DedupSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub threshold: f64,
    pub dry_run: bool,
    pub wing: Option<String>,
    pub source: Option<String>,
    pub min_count: usize,
    pub sources_checked: usize,
    pub total_drawers: usize,
    pub kept: usize,
    pub deleted: usize,
    pub stats_only: bool,
    pub groups: Vec<DedupSourceResult>,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompressSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub version: String,
    pub wing: Option<String>,
    pub dry_run: bool,
    pub processed: usize,
    pub stored: usize,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub compression_ratio: f64,
    pub entries: Vec<CompressedDrawer>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WakeUpSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub version: String,
    pub wing: Option<String>,
    pub identity_path: String,
    pub identity: String,
    pub layer1: String,
    pub token_estimate: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecallSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub version: String,
    pub wing: Option<String>,
    pub room: Option<String>,
    pub n_results: usize,
    pub total_matches: usize,
    pub text: String,
    pub results: Vec<SearchHit>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LayerStatusSummary {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub version: String,
    pub identity_path: String,
    pub identity_exists: bool,
    pub identity_tokens: usize,
    pub total_drawers: usize,
    pub layer0_description: String,
    pub layer1_description: String,
    pub layer2_description: String,
    pub layer3_description: String,
}
