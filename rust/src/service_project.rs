use std::path::Path;

use crate::compression_runtime::CompressionRuntime;
use crate::error::Result;
use crate::init_runtime::InitRuntime;
use crate::miner::mine_project_run;
use crate::model::{CompressSummary, InitSummary, MineProgressEvent, MineRequest, MineSummary};
use crate::service::App;

impl App {
    pub async fn init(&self) -> Result<InitSummary> {
        InitRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .init()
        .await
    }

    pub async fn init_project(&self, project_dir: &Path) -> Result<InitSummary> {
        InitRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .init_project(project_dir)
        .await
    }

    pub async fn mine_project(&self, dir: &Path, request: &MineRequest) -> Result<MineSummary> {
        self.mine_project_with_progress(dir, request, |_| {}).await
    }

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

    pub async fn compress(&self, wing: Option<&str>, dry_run: bool) -> Result<CompressSummary> {
        CompressionRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .compress(wing, dry_run)
    }
}
