use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::registry_research::wikipedia_lookup;
pub use crate::registry_types::{
    COMMON_ENGLISH_WORDS, EntityRegistry, RegistryLearnSummary, RegistryLearnSummaryFields,
    RegistryLookupResult, RegistryPerson, RegistryResearchEntry, RegistrySummary, SeedPerson,
};

#[path = "registry_lookup.rs"]
mod lookup;

impl EntityRegistry {
    pub fn empty(mode: &str) -> Self {
        Self {
            version: 1,
            mode: mode.to_string(),
            people: BTreeMap::new(),
            projects: Vec::new(),
            ambiguous_flags: Vec::new(),
            wiki_cache: BTreeMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::empty("work"))
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn seed(
        &mut self,
        mode: &str,
        people: &[SeedPerson],
        projects: &[String],
        aliases: &BTreeMap<String, String>,
    ) {
        self.mode = mode.to_string();
        self.projects = projects.to_vec();

        let reverse_aliases = aliases
            .iter()
            .map(|(alias, canonical)| (canonical.to_string(), alias.to_string()))
            .collect::<BTreeMap<_, _>>();

        for person in people {
            self.people.insert(
                person.name.clone(),
                RegistryPerson {
                    source: "onboarding".to_string(),
                    contexts: vec![person.context.clone()],
                    aliases: reverse_aliases
                        .get(&person.name)
                        .map(|alias| vec![alias.clone()])
                        .unwrap_or_default(),
                    relationship: person.relationship.clone(),
                    confidence: 1.0,
                    canonical: None,
                },
            );

            if let Some(alias) = reverse_aliases.get(&person.name) {
                self.people.insert(
                    alias.clone(),
                    RegistryPerson {
                        source: "onboarding".to_string(),
                        contexts: vec![person.context.clone()],
                        aliases: vec![person.name.clone()],
                        relationship: person.relationship.clone(),
                        confidence: 1.0,
                        canonical: Some(person.name.clone()),
                    },
                );
            }
        }

        self.recompute_ambiguous_flags();
    }

    pub fn bootstrap(mode: &str, people: &[String], projects: &[String]) -> Self {
        let mut registry = Self::empty(mode);
        for person in people {
            registry.people.insert(
                person.clone(),
                RegistryPerson {
                    source: "bootstrap".to_string(),
                    contexts: vec!["work".to_string()],
                    aliases: Vec::new(),
                    relationship: String::new(),
                    confidence: 1.0,
                    canonical: None,
                },
            );
        }
        registry.projects = projects.to_vec();
        registry.recompute_ambiguous_flags();
        registry
    }

    pub fn learn(&mut self, people: &[String], projects: &[String]) -> RegistryLearnSummaryFields {
        let mut added_people = Vec::new();
        let mut added_projects = Vec::new();

        for person in people {
            if !self
                .people
                .keys()
                .any(|existing| existing.eq_ignore_ascii_case(person))
            {
                self.people.insert(
                    person.clone(),
                    RegistryPerson {
                        source: "learned".to_string(),
                        contexts: vec![self.mode_context()],
                        aliases: Vec::new(),
                        relationship: String::new(),
                        confidence: 0.8,
                        canonical: None,
                    },
                );
                added_people.push(person.clone());
            }
        }

        for project in projects {
            if !self
                .projects
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(project))
            {
                self.projects.push(project.clone());
                added_projects.push(project.clone());
            }
        }

        self.projects.sort();
        self.projects
            .dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        self.recompute_ambiguous_flags();

        RegistryLearnSummaryFields {
            added_people,
            added_projects,
            total_people: self.people.len(),
            total_projects: self.projects.len(),
        }
    }

    pub fn add_person(&mut self, name: &str, relationship: &str, context: &str) {
        self.people.insert(
            name.to_string(),
            RegistryPerson {
                source: "manual".to_string(),
                contexts: vec![context.to_string()],
                aliases: Vec::new(),
                relationship: relationship.to_string(),
                confidence: 1.0,
                canonical: None,
            },
        );
        self.recompute_ambiguous_flags();
    }

    pub fn add_project(&mut self, project: &str) {
        if !self
            .projects
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(project))
        {
            self.projects.push(project.to_string());
            self.projects.sort();
        }
    }

    pub fn add_alias(&mut self, canonical: &str, alias: &str) {
        let cloned = if let Some(person) = self.people.get_mut(canonical) {
            if !person
                .aliases
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(alias))
            {
                person.aliases.push(alias.to_string());
                person.aliases.sort();
            }
            Some((
                person.source.clone(),
                person.contexts.clone(),
                person.relationship.clone(),
                person.confidence,
            ))
        } else {
            None
        };
        if let Some((source, contexts, relationship, confidence)) = cloned {
            self.people.insert(
                alias.to_string(),
                RegistryPerson {
                    source,
                    contexts,
                    aliases: vec![canonical.to_string()],
                    relationship,
                    confidence,
                    canonical: Some(canonical.to_string()),
                },
            );
        }
        self.recompute_ambiguous_flags();
    }
    pub fn summary(&self, registry_path: &Path) -> RegistrySummary {
        let mut people = self.people.keys().cloned().collect::<Vec<_>>();
        people.sort();
        RegistrySummary {
            kind: "registry_summary".to_string(),
            registry_path: registry_path.display().to_string(),
            mode: self.mode.clone(),
            people_count: self.people.len(),
            project_count: self.projects.len(),
            ambiguous_flags: self.ambiguous_flags.clone(),
            people,
            projects: self.projects.clone(),
        }
    }

    pub fn research(&mut self, word: &str, auto_confirm: bool) -> Result<RegistryResearchEntry> {
        if let Some(cached) = self.wiki_cache.get(word) {
            return Ok(cached.clone());
        }

        let mut result = wikipedia_lookup(word)?;
        result.confirmed = auto_confirm;
        self.wiki_cache.insert(word.to_string(), result.clone());
        Ok(result)
    }

    pub fn confirm_research(
        &mut self,
        word: &str,
        entity_type: &str,
        relationship: &str,
        context: &str,
    ) {
        if let Some(cached) = self.wiki_cache.get_mut(word) {
            cached.confirmed = true;
            cached.confirmed_type = Some(entity_type.to_string());
        }

        if entity_type == "person" {
            self.people.insert(
                word.to_string(),
                RegistryPerson {
                    source: "wiki".to_string(),
                    contexts: vec![context.to_string()],
                    aliases: Vec::new(),
                    relationship: relationship.to_string(),
                    confidence: 0.9,
                    canonical: None,
                },
            );
            if COMMON_ENGLISH_WORDS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(word))
                && !self
                    .ambiguous_flags
                    .iter()
                    .any(|flag| flag.eq_ignore_ascii_case(word))
            {
                self.ambiguous_flags.push(word.to_ascii_lowercase());
                self.ambiguous_flags.sort();
                self.ambiguous_flags.dedup();
            }
        }

        self.recompute_ambiguous_flags();
    }

    fn recompute_ambiguous_flags(&mut self) {
        let mut flags = self
            .people
            .keys()
            .filter(|person| {
                COMMON_ENGLISH_WORDS
                    .iter()
                    .any(|word| word.eq_ignore_ascii_case(person))
            })
            .map(|person| person.to_ascii_lowercase())
            .collect::<Vec<_>>();
        flags.sort();
        flags.dedup();
        self.ambiguous_flags = flags;
    }

    fn mode_context(&self) -> String {
        if self.mode == "combo" {
            "personal".to_string()
        } else {
            self.mode.clone()
        }
    }
}

#[cfg(test)]
mod tests {
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
