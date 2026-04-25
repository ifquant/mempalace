//! Project-local runtime facade for registry operations.
//!
//! This file is the audit entrypoint for registry reads, writes, learning, and
//! research once a caller has already chosen the target project directory.

use std::path::{Path, PathBuf};

use crate::entity_detector::detect_entities_for_registry;
use crate::error::Result;
use crate::model::{
    RegistryConfirmResult, RegistryLearnResult, RegistryLookupResult, RegistryQueryResult,
    RegistryResearchResult, RegistrySummaryResult, RegistryWriteResult,
};
use crate::registry::EntityRegistry;

pub struct RegistryRuntime {
    project_dir: PathBuf,
}

impl RegistryRuntime {
    /// Binds registry operations to one project directory.
    pub fn new(project_dir: &Path) -> Self {
        Self {
            project_dir: project_dir.to_path_buf(),
        }
    }

    /// Returns the canonical `entity_registry.json` path for this project.
    pub fn registry_path(&self) -> PathBuf {
        self.project_dir.join("entity_registry.json")
    }

    /// Summarizes the current registry contents.
    pub fn summary(&self) -> Result<RegistrySummaryResult> {
        let registry_path = self.registry_path();
        let summary = EntityRegistry::load(&registry_path)?.summary(&registry_path);
        Ok(RegistrySummaryResult {
            kind: summary.kind,
            registry_path: summary.registry_path,
            mode: summary.mode,
            people_count: summary.people_count,
            project_count: summary.project_count,
            ambiguous_flags: summary.ambiguous_flags,
            people: summary.people,
            projects: summary.projects,
        })
    }

    /// Resolves one word against registry people, projects, and confirmed research cache.
    pub fn lookup(&self, word: &str, context: &str) -> Result<RegistryLookupResult> {
        let registry_path = self.registry_path();
        let lookup = EntityRegistry::load(&registry_path)?.lookup(word, context);
        Ok(RegistryLookupResult {
            kind: "registry_lookup".to_string(),
            registry_path: registry_path.display().to_string(),
            word: lookup.word,
            r#type: lookup.r#type,
            confidence: lookup.confidence,
            source: lookup.source,
            name: lookup.name,
            context: lookup.context,
            needs_disambiguation: lookup.needs_disambiguation,
            disambiguated_by: lookup.disambiguated_by,
        })
    }

    /// Learns additional people/projects by rescanning the project tree.
    pub fn learn(&self) -> Result<RegistryLearnResult> {
        let registry_path = self.registry_path();
        let mut registry = EntityRegistry::load(&registry_path)?;
        let (people, projects) = detect_entities_for_registry(&self.project_dir)?;
        let learned = registry.learn(&people, &projects);
        registry.save(&registry_path)?;
        Ok(RegistryLearnResult {
            kind: "registry_learn".to_string(),
            project_path: self.project_dir.display().to_string(),
            registry_path: registry_path.display().to_string(),
            added_people: learned.added_people,
            added_projects: learned.added_projects,
            total_people: learned.total_people,
            total_projects: learned.total_projects,
        })
    }

    /// Adds one person entry and persists the updated registry.
    pub fn add_person(
        &self,
        name: &str,
        relationship: &str,
        context: &str,
    ) -> Result<RegistryWriteResult> {
        let registry_path = self.registry_path();
        let mut registry = EntityRegistry::load(&registry_path)?;
        registry.add_person(name, relationship, context);
        registry.save(&registry_path)?;
        Ok(RegistryWriteResult {
            kind: "registry_write".to_string(),
            registry_path: registry_path.display().to_string(),
            action: "add_person".to_string(),
            success: true,
            name: name.to_string(),
            canonical: None,
            mode: registry.mode.clone(),
            people_count: registry.people.len(),
            project_count: registry.projects.len(),
        })
    }

    /// Adds one project entry and persists the updated registry.
    pub fn add_project(&self, project: &str) -> Result<RegistryWriteResult> {
        let registry_path = self.registry_path();
        let mut registry = EntityRegistry::load(&registry_path)?;
        registry.add_project(project);
        registry.save(&registry_path)?;
        Ok(RegistryWriteResult {
            kind: "registry_write".to_string(),
            registry_path: registry_path.display().to_string(),
            action: "add_project".to_string(),
            success: true,
            name: project.to_string(),
            canonical: None,
            mode: registry.mode.clone(),
            people_count: registry.people.len(),
            project_count: registry.projects.len(),
        })
    }

    /// Adds an alias entry for an existing canonical person and persists the registry.
    pub fn add_alias(&self, canonical: &str, alias: &str) -> Result<RegistryWriteResult> {
        let registry_path = self.registry_path();
        let mut registry = EntityRegistry::load(&registry_path)?;
        registry.add_alias(canonical, alias);
        registry.save(&registry_path)?;
        Ok(RegistryWriteResult {
            kind: "registry_write".to_string(),
            registry_path: registry_path.display().to_string(),
            action: "add_alias".to_string(),
            success: true,
            name: alias.to_string(),
            canonical: Some(canonical.to_string()),
            mode: registry.mode.clone(),
            people_count: registry.people.len(),
            project_count: registry.projects.len(),
        })
    }

    /// Parses a free-form query into known people and unknown capitalized candidates.
    pub fn query(&self, query: &str) -> Result<RegistryQueryResult> {
        let registry_path = self.registry_path();
        let registry = EntityRegistry::load(&registry_path)?;
        Ok(RegistryQueryResult {
            kind: "registry_query".to_string(),
            registry_path: registry_path.display().to_string(),
            query: query.to_string(),
            people: registry.extract_people_from_query(query),
            unknown_candidates: registry.extract_unknown_candidates(query),
        })
    }

    /// Runs one Wikipedia-backed research lookup and saves the cache entry.
    pub fn research(&self, word: &str, auto_confirm: bool) -> Result<RegistryResearchResult> {
        let registry_path = self.registry_path();
        let mut registry = EntityRegistry::load(&registry_path)?;
        let research = registry.research(word, auto_confirm)?;
        registry.save(&registry_path)?;
        Ok(RegistryResearchResult {
            kind: "registry_research".to_string(),
            registry_path: registry_path.display().to_string(),
            word: research.word,
            inferred_type: research.inferred_type,
            confidence: research.confidence,
            wiki_title: research.wiki_title,
            wiki_summary: research.wiki_summary,
            note: research.note,
            confirmed: research.confirmed,
            confirmed_type: research.confirmed_type,
        })
    }

    /// Confirms a researched term and promotes it into the registry when applicable.
    pub fn confirm_research(
        &self,
        word: &str,
        entity_type: &str,
        relationship: &str,
        context: &str,
    ) -> Result<RegistryConfirmResult> {
        let registry_path = self.registry_path();
        let mut registry = EntityRegistry::load(&registry_path)?;
        registry.confirm_research(word, entity_type, relationship, context);
        registry.save(&registry_path)?;
        Ok(RegistryConfirmResult {
            kind: "registry_confirm".to_string(),
            registry_path: registry_path.display().to_string(),
            word: word.to_string(),
            entity_type: entity_type.to_string(),
            relationship: relationship.to_string(),
            context: context.to_string(),
            total_people: registry.people.len(),
            total_projects: registry.projects.len(),
            wiki_cache_entries: registry.wiki_cache.len(),
        })
    }
}
