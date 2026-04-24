use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const COMMON_ENGLISH_WORDS: &[&str] = &[
    "ever",
    "grace",
    "will",
    "bill",
    "mark",
    "april",
    "may",
    "june",
    "joy",
    "hope",
    "faith",
    "chance",
    "chase",
    "hunter",
    "dash",
    "flash",
    "star",
    "sky",
    "river",
    "brook",
    "lane",
    "art",
    "clay",
    "gil",
    "nat",
    "max",
    "rex",
    "ray",
    "jay",
    "rose",
    "violet",
    "lily",
    "ivy",
    "ash",
    "reed",
    "sage",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "january",
    "february",
    "march",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryPerson {
    pub source: String,
    pub contexts: Vec<String>,
    pub aliases: Vec<String>,
    pub relationship: String,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryResearchEntry {
    pub word: String,
    pub inferred_type: String,
    pub confidence: f64,
    pub wiki_summary: Option<String>,
    pub wiki_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub confirmed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_type: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntityRegistry {
    pub version: u8,
    pub mode: String,
    pub people: BTreeMap<String, RegistryPerson>,
    pub projects: Vec<String>,
    pub ambiguous_flags: Vec<String>,
    pub wiki_cache: BTreeMap<String, RegistryResearchEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistryLookupResult {
    pub word: String,
    pub r#type: String,
    pub confidence: f64,
    pub source: String,
    pub name: String,
    pub context: Vec<String>,
    pub needs_disambiguation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disambiguated_by: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RegistrySummary {
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
pub struct RegistryLearnSummary {
    pub kind: String,
    pub project_path: String,
    pub registry_path: String,
    pub added_people: Vec<String>,
    pub added_projects: Vec<String>,
    pub total_people: usize,
    pub total_projects: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeedPerson {
    pub name: String,
    pub relationship: String,
    pub context: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegistryLearnSummaryFields {
    pub added_people: Vec<String>,
    pub added_projects: Vec<String>,
    pub total_people: usize,
    pub total_projects: usize,
}
