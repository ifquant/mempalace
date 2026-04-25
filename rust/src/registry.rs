//! Entity registry facade for project-local people, aliases, and research cache.
//!
//! Audit readers can start here to see the split between persistence (`registry_io`),
//! lookup/disambiguation (`registry_lookup`), and write-side updates
//! (`registry_mutation`).

pub use crate::registry_types::{
    COMMON_ENGLISH_WORDS, EntityRegistry, RegistryLearnSummary, RegistryLearnSummaryFields,
    RegistryLookupResult, RegistryPerson, RegistryResearchEntry, RegistrySummary, SeedPerson,
};

#[path = "registry_io.rs"]
mod io;
#[path = "registry_lookup.rs"]
mod lookup;
#[path = "registry_mutation.rs"]
mod mutation;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use tempfile::tempdir;

    #[test]
    fn registry_load_save_round_trip() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("entity_registry.json");
        let mut registry = EntityRegistry::empty("work");
        registry.seed(
            "work",
            &[SeedPerson {
                name: "Jordan".to_string(),
                relationship: "coworker".to_string(),
                context: "work".to_string(),
            }],
            &["Atlas".to_string()],
            &BTreeMap::new(),
        );
        registry.save(&path).unwrap();

        let loaded = EntityRegistry::load(&path).unwrap();
        assert!(loaded.people.contains_key("Jordan"));
        assert_eq!(loaded.projects, vec!["Atlas".to_string()]);
    }

    #[test]
    fn confirm_research_promotes_person_into_registry() {
        let mut registry = EntityRegistry::empty("personal");
        registry.wiki_cache.insert(
            "Riley".to_string(),
            RegistryResearchEntry {
                word: "Riley".to_string(),
                inferred_type: "person".to_string(),
                confidence: 0.9,
                wiki_summary: Some("riley is a given name".to_string()),
                wiki_title: Some("Riley".to_string()),
                note: None,
                confirmed: false,
                confirmed_type: None,
            },
        );

        registry.confirm_research("Riley", "person", "daughter", "personal");

        assert_eq!(registry.people["Riley"].source, "wiki");
        assert!(registry.wiki_cache["Riley"].confirmed);
        assert_eq!(
            registry.wiki_cache["Riley"].confirmed_type.as_deref(),
            Some("person")
        );
    }
}
