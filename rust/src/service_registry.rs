//! Registry-oriented `App` helpers for project-local people/project knowledge.
//!
//! Unlike palace operations, these methods work directly against the source
//! project's registry files through `RegistryRuntime`.

use std::path::Path;

use crate::error::Result;
use crate::model::{
    RegistryConfirmResult, RegistryLearnResult, RegistryLookupResult, RegistryQueryResult,
    RegistryResearchResult, RegistrySummaryResult, RegistryWriteResult,
};
use crate::registry_runtime::RegistryRuntime;
use crate::service::App;

impl App {
    /// Summarize the current project registry state.
    pub fn registry_summary(&self, project_dir: &Path) -> Result<RegistrySummaryResult> {
        RegistryRuntime::new(project_dir).summary()
    }

    /// Resolve one term against the registry with caller-provided context.
    pub fn registry_lookup(
        &self,
        project_dir: &Path,
        word: &str,
        context: &str,
    ) -> Result<RegistryLookupResult> {
        RegistryRuntime::new(project_dir).lookup(word, context)
    }

    /// Learn registry entities from the current project tree.
    pub fn registry_learn(&self, project_dir: &Path) -> Result<RegistryLearnResult> {
        RegistryRuntime::new(project_dir).learn()
    }

    /// Add a person record to the project registry.
    pub fn registry_add_person(
        &self,
        project_dir: &Path,
        name: &str,
        relationship: &str,
        context: &str,
    ) -> Result<RegistryWriteResult> {
        RegistryRuntime::new(project_dir).add_person(name, relationship, context)
    }

    /// Add a project record to the project registry.
    pub fn registry_add_project(
        &self,
        project_dir: &Path,
        project: &str,
    ) -> Result<RegistryWriteResult> {
        RegistryRuntime::new(project_dir).add_project(project)
    }

    /// Record an alias for an existing canonical entity.
    pub fn registry_add_alias(
        &self,
        project_dir: &Path,
        canonical: &str,
        alias: &str,
    ) -> Result<RegistryWriteResult> {
        RegistryRuntime::new(project_dir).add_alias(canonical, alias)
    }

    /// Run a broader free-text query over the registry model.
    pub fn registry_query(&self, project_dir: &Path, query: &str) -> Result<RegistryQueryResult> {
        RegistryRuntime::new(project_dir).query(query)
    }

    /// Gather research suggestions for an unknown term before confirmation.
    pub fn registry_research(
        &self,
        project_dir: &Path,
        word: &str,
        auto_confirm: bool,
    ) -> Result<RegistryResearchResult> {
        RegistryRuntime::new(project_dir).research(word, auto_confirm)
    }

    /// Confirm one researched entity back into the registry files.
    pub fn registry_confirm_research(
        &self,
        project_dir: &Path,
        word: &str,
        entity_type: &str,
        relationship: &str,
        context: &str,
    ) -> Result<RegistryConfirmResult> {
        RegistryRuntime::new(project_dir).confirm_research(word, entity_type, relationship, context)
    }
}
