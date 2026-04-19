use std::path::Path;

use crate::VERSION;
use crate::bootstrap::bootstrap_project;
use crate::config::AppConfig;
use crate::embed::EmbeddingProvider;
use crate::error::Result;
use crate::model::InitSummary;
use crate::palace::ensure_vector_store;
use crate::storage::sqlite::{CURRENT_SCHEMA_VERSION, SqliteStore};

pub struct InitRuntime<'a> {
    pub config: &'a AppConfig,
    pub embedder: &'a dyn EmbeddingProvider,
}

impl<'a> InitRuntime<'a> {
    async fn prepare_storage(&self) -> Result<i64> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let _vector = ensure_vector_store(self.config, self.embedder.profile()).await?;
        Ok(sqlite.schema_version()?.unwrap_or(CURRENT_SCHEMA_VERSION))
    }

    pub async fn init(&self) -> Result<InitSummary> {
        let schema_version = self.prepare_storage().await?;
        Ok(InitSummary {
            kind: "init".to_string(),
            project_path: self.config.palace_path.display().to_string(),
            wing: "general".to_string(),
            configured_rooms: vec!["general".to_string()],
            detected_people: Vec::new(),
            detected_projects: Vec::new(),
            config_path: None,
            config_written: false,
            entities_path: None,
            entities_written: false,
            entity_registry_path: None,
            entity_registry_written: false,
            aaak_entities_path: None,
            aaak_entities_written: false,
            critical_facts_path: None,
            critical_facts_written: false,
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            schema_version,
        })
    }

    pub async fn init_project(&self, project_dir: &Path) -> Result<InitSummary> {
        let schema_version = self.prepare_storage().await?;
        let bootstrap = bootstrap_project(project_dir)?;

        Ok(InitSummary {
            kind: "init".to_string(),
            project_path: project_dir.display().to_string(),
            wing: bootstrap.wing,
            configured_rooms: bootstrap.configured_rooms,
            detected_people: bootstrap.detected_people,
            detected_projects: bootstrap.detected_projects,
            config_path: bootstrap.config_path,
            config_written: bootstrap.config_written,
            entities_path: bootstrap.entities_path,
            entities_written: bootstrap.entities_written,
            entity_registry_path: bootstrap.entity_registry_path,
            entity_registry_written: bootstrap.entity_registry_written,
            aaak_entities_path: bootstrap.aaak_entities_path,
            aaak_entities_written: bootstrap.aaak_entities_written,
            critical_facts_path: bootstrap.critical_facts_path,
            critical_facts_written: bootstrap.critical_facts_written,
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            schema_version,
        })
    }
}
