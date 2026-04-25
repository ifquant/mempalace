//! Write-side mutation helpers for the entity registry.
//!
//! These methods keep the JSON snapshot coherent after learn/manual/research
//! updates and recompute ambiguous-name flags whenever person identities change.

use crate::registry_types::{
    COMMON_ENGLISH_WORDS, EntityRegistry, RegistryLearnSummaryFields, RegistryPerson,
};

impl EntityRegistry {
    /// Adds newly detected people/projects without overwriting existing explicit entries.
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

    /// Adds or replaces one person entry with manual provenance.
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

    /// Adds one project name if it is not already present case-insensitively.
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

    /// Registers an alias row that points back to an existing canonical person.
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
            // Promoting research creates the same top-level person record shape as
            // manual/onboarding entries so later lookups do not need a special case.
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

    pub(crate) fn recompute_ambiguous_flags(&mut self) {
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

    /// Resolves the default learned context for the current registry mode.
    fn mode_context(&self) -> String {
        if self.mode == "combo" {
            "personal".to_string()
        } else {
            self.mode.clone()
        }
    }
}
