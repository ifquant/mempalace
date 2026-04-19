use crate::embedding_runtime::EmbeddingRuntime;
use crate::error::Result;
use crate::maintenance_runtime::MaintenanceRuntime;
use crate::model::{
    DedupSummary, DoctorSummary, MigrateSummary, PrepareEmbeddingSummary, RepairPruneSummary,
    RepairRebuildSummary, RepairScanSummary, RepairSummary,
};
use crate::service::App;

impl App {
    pub async fn migrate(&self) -> Result<MigrateSummary> {
        MaintenanceRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .migrate()
        .await
    }

    pub async fn repair(&self) -> Result<RepairSummary> {
        MaintenanceRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .repair()
        .await
    }

    pub async fn repair_scan(&self, wing: Option<&str>) -> Result<RepairScanSummary> {
        MaintenanceRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .repair_scan(wing)
        .await
    }

    pub async fn repair_prune(&self, confirm: bool) -> Result<RepairPruneSummary> {
        MaintenanceRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .repair_prune(confirm)
        .await
    }

    pub async fn repair_rebuild(&self) -> Result<RepairRebuildSummary> {
        MaintenanceRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .repair_rebuild()
        .await
    }

    pub async fn dedup(
        &self,
        threshold: f64,
        dry_run: bool,
        wing: Option<&str>,
        source_pattern: Option<&str>,
        min_count: usize,
        stats_only: bool,
    ) -> Result<DedupSummary> {
        MaintenanceRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .dedup(
            threshold,
            dry_run,
            wing,
            source_pattern,
            min_count,
            stats_only,
        )
        .await
    }

    pub async fn doctor(&self, warm_embedding: bool) -> Result<DoctorSummary> {
        EmbeddingRuntime {
            config: self.config.clone(),
            embedder: self.embedder.clone(),
        }
        .doctor(warm_embedding)
    }

    pub async fn prepare_embedding(
        &self,
        attempts: usize,
        wait_ms: u64,
    ) -> Result<PrepareEmbeddingSummary> {
        EmbeddingRuntime {
            config: self.config.clone(),
            embedder: self.embedder.clone(),
        }
        .prepare_embedding(attempts, wait_ms)
        .await
    }
}
