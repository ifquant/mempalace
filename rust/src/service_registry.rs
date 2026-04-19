use std::path::Path;

use crate::error::Result;
use crate::model::{
    RegistryConfirmResult, RegistryLearnResult, RegistryLookupResult, RegistryQueryResult,
    RegistryResearchResult, RegistrySummaryResult, RegistryWriteResult,
};
use crate::registry_runtime::RegistryRuntime;
use crate::service::App;

impl App {
    pub fn registry_summary(&self, project_dir: &Path) -> Result<RegistrySummaryResult> {
        RegistryRuntime::new(project_dir).summary()
    }

    pub fn registry_lookup(
        &self,
        project_dir: &Path,
        word: &str,
        context: &str,
    ) -> Result<RegistryLookupResult> {
        RegistryRuntime::new(project_dir).lookup(word, context)
    }

    pub fn registry_learn(&self, project_dir: &Path) -> Result<RegistryLearnResult> {
        RegistryRuntime::new(project_dir).learn()
    }

    pub fn registry_add_person(
        &self,
        project_dir: &Path,
        name: &str,
        relationship: &str,
        context: &str,
    ) -> Result<RegistryWriteResult> {
        RegistryRuntime::new(project_dir).add_person(name, relationship, context)
    }

    pub fn registry_add_project(
        &self,
        project_dir: &Path,
        project: &str,
    ) -> Result<RegistryWriteResult> {
        RegistryRuntime::new(project_dir).add_project(project)
    }

    pub fn registry_add_alias(
        &self,
        project_dir: &Path,
        canonical: &str,
        alias: &str,
    ) -> Result<RegistryWriteResult> {
        RegistryRuntime::new(project_dir).add_alias(canonical, alias)
    }

    pub fn registry_query(&self, project_dir: &Path, query: &str) -> Result<RegistryQueryResult> {
        RegistryRuntime::new(project_dir).query(query)
    }

    pub fn registry_research(
        &self,
        project_dir: &Path,
        word: &str,
        auto_confirm: bool,
    ) -> Result<RegistryResearchResult> {
        RegistryRuntime::new(project_dir).research(word, auto_confirm)
    }

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
