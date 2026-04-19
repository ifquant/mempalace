use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::palace::SearchFilters;

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
    pub files_processed: usize,
    pub files_mined: usize,
    pub drawers_added: usize,
    pub files_skipped: usize,
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
    DryRunSummary {
        file_name: String,
        summary: String,
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
    pub project_path: String,
    pub wing: String,
    pub configured_rooms: Vec<String>,
    pub detected_people: Vec<String>,
    pub detected_projects: Vec<String>,
    pub config_path: Option<String>,
    pub config_written: bool,
    pub entities_path: Option<String>,
    pub entities_written: bool,
    pub entity_registry_path: Option<String>,
    pub entity_registry_written: bool,
    pub aaak_entities_path: Option<String>,
    pub aaak_entities_written: bool,
    pub critical_facts_path: Option<String>,
    pub critical_facts_written: bool,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub schema_version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OnboardingSummary {
    pub kind: String,
    pub project_path: String,
    pub mode: String,
    pub wing: String,
    pub wings: Vec<String>,
    pub people: Vec<String>,
    pub projects: Vec<String>,
    pub aliases: BTreeMap<String, String>,
    pub ambiguous_flags: Vec<String>,
    pub auto_detected_people: Vec<String>,
    pub auto_detected_projects: Vec<String>,
    pub config_path: Option<String>,
    pub config_written: bool,
    pub entities_path: Option<String>,
    pub entities_written: bool,
    pub entity_registry_path: String,
    pub entity_registry_written: bool,
    pub aaak_entities_path: String,
    pub aaak_entities_written: bool,
    pub critical_facts_path: String,
    pub critical_facts_written: bool,
    pub version: String,
}
