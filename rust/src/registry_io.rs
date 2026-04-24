use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::registry_research::wikipedia_lookup;
use crate::registry_types::{EntityRegistry, RegistryResearchEntry, RegistrySummary, SeedPerson};

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
            Ok(Self::empty("personal"))
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn bootstrap(mode: &str, people: &[String], projects: &[String]) -> Self {
        let mut registry = Self::empty(mode);
        for person in people {
            registry.people.insert(
                person.clone(),
                crate::registry_types::RegistryPerson {
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
            let name = person.name.trim();
            if name.is_empty() {
                continue;
            }

            self.people.insert(
                name.to_string(),
                crate::registry_types::RegistryPerson {
                    source: "onboarding".to_string(),
                    contexts: vec![person.context.clone()],
                    aliases: reverse_aliases
                        .get(name)
                        .map(|alias| vec![alias.clone()])
                        .unwrap_or_default(),
                    relationship: person.relationship.clone(),
                    confidence: 1.0,
                    canonical: None,
                },
            );

            if let Some(alias) = reverse_aliases.get(name) {
                self.people.insert(
                    alias.clone(),
                    crate::registry_types::RegistryPerson {
                        source: "onboarding".to_string(),
                        contexts: vec![person.context.clone()],
                        aliases: vec![name.to_string()],
                        relationship: person.relationship.clone(),
                        confidence: 1.0,
                        canonical: Some(name.to_string()),
                    },
                );
            }
        }

        self.recompute_ambiguous_flags();
    }
}
