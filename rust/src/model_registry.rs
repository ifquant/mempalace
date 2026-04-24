use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryLookupResult {
    pub kind: String,
    pub registry_path: String,
    pub word: String,
    pub r#type: String,
    pub confidence: f64,
    pub source: String,
    pub name: String,
    pub context: Vec<String>,
    pub needs_disambiguation: bool,
    pub disambiguated_by: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistrySummaryResult {
    pub kind: String,
    pub registry_path: String,
    pub mode: String,
    pub people_count: usize,
    pub project_count: usize,
    pub ambiguous_flags: Vec<String>,
    pub people: Vec<String>,
    pub projects: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryLearnResult {
    pub kind: String,
    pub project_path: String,
    pub registry_path: String,
    pub added_people: Vec<String>,
    pub added_projects: Vec<String>,
    pub total_people: usize,
    pub total_projects: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryWriteResult {
    pub kind: String,
    pub registry_path: String,
    pub action: String,
    pub success: bool,
    pub name: String,
    pub canonical: Option<String>,
    pub mode: String,
    pub people_count: usize,
    pub project_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryQueryResult {
    pub kind: String,
    pub registry_path: String,
    pub query: String,
    pub people: Vec<String>,
    pub unknown_candidates: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryResearchResult {
    pub kind: String,
    pub registry_path: String,
    pub word: String,
    pub inferred_type: String,
    pub confidence: f64,
    pub wiki_title: Option<String>,
    pub wiki_summary: Option<String>,
    pub note: Option<String>,
    pub confirmed: bool,
    pub confirmed_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryConfirmResult {
    pub kind: String,
    pub registry_path: String,
    pub word: String,
    pub entity_type: String,
    pub relationship: String,
    pub context: String,
    pub total_people: usize,
    pub total_projects: usize,
    pub wiki_cache_entries: usize,
}
