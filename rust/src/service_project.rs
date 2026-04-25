//! Project-facing `App` helpers for bootstrap, mining, and compression flows.
//!
//! These are the orchestration entrypoints used by the CLI when work starts
//! from a source project or transcript file instead of from an existing palace
//! query.

use std::path::Path;

use crate::compression_runtime::CompressionRuntime;
use crate::error::Result;
use crate::init_runtime::InitRuntime;
use crate::miner::mine_project_run;
use crate::model::{CompressSummary, InitSummary, MineProgressEvent, MineRequest, MineSummary};
use crate::service::App;

impl App {
    /// Initialize the configured palace location if it is missing.
    pub async fn init(&self) -> Result<InitSummary> {
        InitRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .init()
        .await
    }

    /// Create project-local bootstrap artifacts for one source directory.
    pub async fn init_project(&self, project_dir: &Path) -> Result<InitSummary> {
        InitRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .init_project(project_dir)
        .await
    }

    /// Mine a project with the default no-op progress callback.
    pub async fn mine_project(&self, dir: &Path, request: &MineRequest) -> Result<MineSummary> {
        self.mine_project_with_progress(dir, request, |_| {}).await
    }

    /// Mine a project while streaming progress events back to the caller.
    ///
    /// The palace is initialized first so the mining pipeline can assume its
    /// storage backends already exist.
    pub async fn mine_project_with_progress<F>(
        &self,
        dir: &Path,
        request: &MineRequest,
        on_progress: F,
    ) -> Result<MineSummary>
    where
        F: FnMut(MineProgressEvent),
    {
        self.init().await?;
        mine_project_run(
            &self.config,
            self.embedder.clone(),
            dir,
            request,
            on_progress,
        )
        .await
    }

    /// Compress stored drawers into higher-level AAAK summaries.
    pub async fn compress(&self, wing: Option<&str>, dry_run: bool) -> Result<CompressSummary> {
        CompressionRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .compress(wing, dry_run)
    }
}
