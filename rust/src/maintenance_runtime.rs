//! Maintenance runtime for schema upgrades, repair, dedup, and rebuild flows.
//!
//! This is the highest-risk storage boundary in the crate. It coordinates
//! migrations plus repair actions that may inspect, preview, delete, or
//! rebuild data across both SQLite and LanceDB.

use crate::VERSION;
use crate::config::AppConfig;
use crate::dedup::{DedupSummaryContext, Deduplicator};
use crate::drawers::drawer_input_from_record;
use crate::embed::EmbeddingProvider;
use crate::error::Result;
use crate::model::{
    DedupSummary, MigrateSummary, RepairPruneSummary, RepairRebuildSummary, RepairScanSummary,
    RepairSummary,
};
use crate::repair::{RepairContext, RepairDiagnostics, backup_sqlite_source, read_corrupt_ids};
use crate::storage::sqlite::{CURRENT_SCHEMA_VERSION, SqliteStore};
use crate::storage::vector::VectorStore;

/// Maintenance facade shared by CLI, MCP, and integration tests.
pub struct MaintenanceRuntime<'a> {
    pub config: &'a AppConfig,
    pub embedder: &'a dyn EmbeddingProvider,
}

impl<'a> MaintenanceRuntime<'a> {
    fn open_sqlite(&self) -> Result<SqliteStore> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        Ok(sqlite)
    }

    fn repair_context(&self) -> RepairContext {
        RepairContext {
            palace_path: self.config.palace_path.clone(),
            sqlite_path: self.config.sqlite_path(),
            lance_path: self.config.lance_path(),
            version: VERSION.to_string(),
        }
    }

    /// Applies pending SQLite schema migrations and reports whether anything changed.
    pub async fn migrate(&self) -> Result<MigrateSummary> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        let schema_version_before = sqlite.schema_version()?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let schema_version_after = sqlite.schema_version()?.unwrap_or(CURRENT_SCHEMA_VERSION);

        Ok(MigrateSummary {
            kind: "migrate".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            schema_version_before,
            schema_version_after,
            changed: schema_version_before != Some(schema_version_after),
        })
    }

    /// Reports whether the palace stores are present, readable, and profile-compatible.
    pub async fn repair(&self) -> Result<RepairSummary> {
        let context = self.repair_context();
        let sqlite_path = self.config.sqlite_path();
        let lance_path = self.config.lance_path();
        let sqlite_exists = sqlite_path.exists();
        let lance_exists = lance_path.exists();
        let mut issues = Vec::new();

        if !sqlite_exists {
            issues.push("SQLite palace file is missing".to_string());
        }
        if !lance_exists {
            issues.push("LanceDB directory is missing".to_string());
        }

        let mut schema_version = None;
        let mut sqlite_drawer_count = None;
        let mut embedding_provider = None;
        let mut embedding_model = None;
        let mut embedding_dimension = None;

        if sqlite_exists {
            let sqlite = SqliteStore::open(&sqlite_path)?;
            sqlite.init_schema()?;
            schema_version = sqlite.schema_version()?;
            sqlite_drawer_count = Some(sqlite.total_drawers()?);
            embedding_provider = sqlite.meta("embedding_provider")?;
            embedding_model = sqlite.meta("embedding_model")?;
            embedding_dimension = sqlite
                .meta("embedding_dimension")?
                .and_then(|value| value.parse::<usize>().ok());

            if let Err(err) = sqlite.ensure_embedding_profile(self.embedder.profile()) {
                issues.push(format!("Embedding profile mismatch: {err}"));
            }
        }

        let vector_accessible = if lance_exists {
            match VectorStore::connect(&lance_path).await {
                Ok(vector) => vector
                    .ensure_table(self.embedder.profile().dimension)
                    .await
                    .map(|_| true)
                    .unwrap_or_else(|err| {
                        issues.push(format!("LanceDB access failed: {err}"));
                        false
                    }),
                Err(err) => {
                    issues.push(format!("LanceDB connect failed: {err}"));
                    false
                }
            }
        } else {
            false
        };

        Ok(context.build_summary(RepairDiagnostics {
            sqlite_exists,
            lance_exists,
            schema_version,
            sqlite_drawer_count,
            embedding_provider,
            embedding_model,
            embedding_dimension,
            vector_accessible,
            issues,
        }))
    }

    /// Scans for drift between SQLite and LanceDB, then stages prune candidates.
    pub async fn repair_scan(&self, wing: Option<&str>) -> Result<RepairScanSummary> {
        let sqlite = self.open_sqlite()?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        let sqlite_drawers = sqlite.list_drawers(wing)?;
        let vector_drawers = vector
            .list_drawers(self.embedder.profile().dimension, wing, None)
            .await?;
        Ok(self
            .repair_context()
            .build_scan_summary(wing, &sqlite_drawers, &vector_drawers)?)
    }

    /// Deletes staged corrupt IDs when `confirm` is true, otherwise returns a preview.
    pub async fn repair_prune(&self, confirm: bool) -> Result<RepairPruneSummary> {
        let context = self.repair_context();
        let queued_ids = read_corrupt_ids(&context.corrupt_ids_path())?;

        if !confirm {
            return Ok(context.build_prune_preview(&queued_ids, confirm));
        }

        let sqlite = self.open_sqlite()?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;

        let mut deleted_from_sqlite = 0usize;
        let mut deleted_from_vector = 0usize;
        let mut failed = 0usize;

        let sqlite_existing = queued_ids
            .iter()
            .map(|drawer_id| sqlite.drawer_exists(drawer_id))
            .collect::<Result<Vec<_>>>()?;
        let vector_existing = {
            let mut existing = Vec::with_capacity(queued_ids.len());
            for drawer_id in &queued_ids {
                existing.push(
                    vector
                        .drawer_exists(self.embedder.profile().dimension, drawer_id)
                        .await?,
                );
            }
            existing
        };

        match sqlite.delete_drawers(&queued_ids) {
            Ok(count) => deleted_from_sqlite = count,
            Err(_) => {
                // Fall back to one-by-one cleanup so one bad row does not block
                // pruning the remaining staged IDs.
                for (drawer_id, existed) in queued_ids.iter().zip(sqlite_existing.iter().copied()) {
                    if !existed {
                        continue;
                    }
                    match sqlite.delete_drawers(std::slice::from_ref(drawer_id)) {
                        Ok(1) => deleted_from_sqlite += 1,
                        _ => failed += 1,
                    }
                }
            }
        }

        match vector
            .delete_drawers(self.embedder.profile().dimension, &queued_ids)
            .await
        {
            Ok(_) => {
                deleted_from_vector = vector_existing.iter().filter(|exists| **exists).count();
                failed += queued_ids.len() - deleted_from_vector;
            }
            Err(_) => {
                for (drawer_id, existed) in queued_ids.iter().zip(vector_existing.iter().copied()) {
                    if !existed {
                        failed += 1;
                        continue;
                    }
                    match vector
                        .delete_drawers(
                            self.embedder.profile().dimension,
                            std::slice::from_ref(drawer_id),
                        )
                        .await
                    {
                        Ok(1) => deleted_from_vector += 1,
                        _ => failed += 1,
                    }
                }
            }
        }

        Ok(context.build_prune_result(
            &queued_ids,
            confirm,
            deleted_from_vector,
            deleted_from_sqlite,
            failed,
        ))
    }

    /// Rebuilds the LanceDB table from SQLite drawers after taking a SQLite backup.
    pub async fn repair_rebuild(&self) -> Result<RepairRebuildSummary> {
        let sqlite_path = self.config.sqlite_path();
        let sqlite = self.open_sqlite()?;
        let drawers = sqlite.list_drawers(None)?;

        let backup_path = backup_sqlite_source(&sqlite_path)?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        // Rebuild is intentionally "clear then repopulate" so the vector side
        // reflects only the canonical SQLite drawer set.
        vector
            .clear_table(self.embedder.profile().dimension)
            .await?;

        let mut rebuilt = 0usize;
        for batch in drawers.chunks(128) {
            let texts = batch
                .iter()
                .map(|drawer| drawer.text.clone())
                .collect::<Vec<_>>();
            let embeddings = self.embedder.embed_documents(&texts)?;
            let inputs = batch
                .iter()
                .map(drawer_input_from_record)
                .collect::<Vec<_>>();
            vector.add_drawers(&inputs, &embeddings).await?;
            rebuilt += inputs.len();
        }

        Ok(self
            .repair_context()
            .build_rebuild_summary(drawers.len(), rebuilt, backup_path))
    }

    /// Plans or executes semantic deduplication within the selected slice.
    pub async fn dedup(
        &self,
        threshold: f64,
        dry_run: bool,
        wing: Option<&str>,
        source_pattern: Option<&str>,
        min_count: usize,
        stats_only: bool,
    ) -> Result<DedupSummary> {
        let sqlite = self.open_sqlite()?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;

        let sqlite_drawers = sqlite.list_drawers(wing)?;
        let vector_drawers = vector
            .list_drawers(self.embedder.profile().dimension, wing, source_pattern)
            .await?;
        let plan = Deduplicator::new(&sqlite_drawers, &vector_drawers).plan(
            threshold,
            source_pattern,
            min_count,
        );

        if !dry_run && !stats_only && !plan.delete_ids.is_empty() {
            // Live dedup deletes from both stores; preview and stats modes stop
            // before any destructive work so reviewers can inspect the plan.
            sqlite.delete_drawers(&plan.delete_ids)?;
            vector
                .delete_drawers(self.embedder.profile().dimension, &plan.delete_ids)
                .await?;
        }

        Ok(plan.into_summary(DedupSummaryContext {
            kind: "dedup".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            threshold,
            dry_run,
            wing: wing.map(ToOwned::to_owned),
            source: source_pattern.map(ToOwned::to_owned),
            min_count,
            stats_only,
        }))
    }
}
