use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::compression_runtime::CompressionRuntime;
use crate::config::AppConfig;
use crate::embed::{EmbeddingProvider, build_embedder};
use crate::embedding_runtime::EmbeddingRuntime;
use crate::error::Result;
use crate::init_runtime::InitRuntime;
use crate::maintenance_runtime::MaintenanceRuntime;
use crate::miner::mine_project_run;
use crate::model::{
    CompressSummary, DedupSummary, DiaryReadResult, DiaryWriteResult, DoctorSummary,
    DrawerDeleteResult, DrawerWriteResult, GraphStats, GraphTraversalResult, InitSummary,
    KgInvalidateResult, KgQueryResult, KgStats, KgTimelineResult, KgTriple, KgWriteResult,
    LayerStatusSummary, MigrateSummary, MineProgressEvent, MineRequest, MineSummary,
    PrepareEmbeddingSummary, RecallSummary, RegistryConfirmResult, RegistryLearnResult,
    RegistryLookupResult, RegistryQueryResult, RegistryResearchResult, RegistrySummaryResult,
    RegistryWriteResult, RepairPruneSummary, RepairRebuildSummary, RepairScanSummary,
    RepairSummary, Rooms, SearchResults, Status, Taxonomy, TunnelRoom, WakeUpSummary,
};
use crate::palace_ops::PalaceOpsRuntime;
use crate::palace_read::PalaceReadRuntime;
use crate::registry_runtime::RegistryRuntime;

#[derive(Clone)]
pub struct App {
    pub config: AppConfig,
    embedder: Arc<dyn EmbeddingProvider>,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self> {
        let embedder = build_embedder(&config.embedding)?;
        Ok(Self { config, embedder })
    }

    pub fn with_embedder(config: AppConfig, embedder: Arc<dyn EmbeddingProvider>) -> Self {
        Self { config, embedder }
    }

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

    pub async fn status(&self) -> Result<Status> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .status()
        .await
    }

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

    pub async fn list_wings(&self) -> Result<BTreeMap<String, usize>> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .list_wings()
        .await
    }

    pub async fn list_rooms(&self, wing: Option<&str>) -> Result<Rooms> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .list_rooms(wing)
        .await
    }

    pub async fn taxonomy(&self) -> Result<Taxonomy> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .taxonomy()
        .await
    }

    pub async fn traverse_graph(
        &self,
        start_room: &str,
        max_hops: usize,
    ) -> Result<GraphTraversalResult> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .traverse_graph(start_room, max_hops)
        .await
    }

    pub async fn find_tunnels(
        &self,
        wing_a: Option<&str>,
        wing_b: Option<&str>,
    ) -> Result<Vec<TunnelRoom>> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .find_tunnels(wing_a, wing_b)
        .await
    }

    pub async fn graph_stats(&self) -> Result<GraphStats> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .graph_stats()
        .await
    }

    pub async fn search(
        &self,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Result<SearchResults> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .search(query, wing, room, limit)
        .await
    }

    pub async fn compress(&self, wing: Option<&str>, dry_run: bool) -> Result<CompressSummary> {
        CompressionRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .compress(wing, dry_run)
    }

    pub async fn wake_up(&self, wing: Option<&str>) -> Result<WakeUpSummary> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .wake_up(wing)
        .await
    }

    pub async fn recall(
        &self,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> Result<RecallSummary> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .recall(wing, room, n_results)
        .await
    }

    pub async fn layer_status(&self) -> Result<LayerStatusSummary> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .layer_status()
        .await
    }

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

    pub async fn add_kg_triple(&self, triple: &KgTriple) -> Result<()> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .add_kg_triple(triple)
        .await
    }

    pub async fn query_kg(&self, subject: &str) -> Result<Vec<KgTriple>> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .query_kg_raw(subject)
        .await
    }

    pub async fn kg_query(
        &self,
        entity: &str,
        as_of: Option<&str>,
        direction: &str,
    ) -> Result<KgQueryResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_query(entity, as_of, direction)
        .await
    }

    pub async fn kg_timeline(&self, entity: Option<&str>) -> Result<KgTimelineResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_timeline(entity)
        .await
    }

    pub async fn kg_stats(&self) -> Result<KgStats> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_stats()
        .await
    }

    pub async fn kg_add(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        valid_from: Option<&str>,
    ) -> Result<KgWriteResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_add(subject, predicate, object, valid_from)
        .await
    }

    pub async fn kg_invalidate(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        ended: Option<&str>,
    ) -> Result<KgInvalidateResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_invalidate(subject, predicate, object, ended)
        .await
    }

    pub async fn add_drawer(
        &self,
        wing: &str,
        room: &str,
        content: &str,
        source_file: Option<&str>,
        added_by: Option<&str>,
    ) -> Result<DrawerWriteResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .add_drawer(wing, room, content, source_file, added_by)
        .await
    }

    pub async fn delete_drawer(&self, drawer_id: &str) -> Result<DrawerDeleteResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .delete_drawer(drawer_id)
        .await
    }

    pub async fn diary_write(
        &self,
        agent_name: &str,
        entry: &str,
        topic: &str,
    ) -> Result<DiaryWriteResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .diary_write(agent_name, entry, topic)
        .await
    }

    pub async fn diary_read(&self, agent_name: &str, last_n: usize) -> Result<DiaryReadResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .diary_read(agent_name, last_n)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::miner_support::chunk_text;
    use crate::model::SearchHit;
    use crate::room_detector::detect_room;
    use crate::searcher::normalize_search_hits;

    #[test]
    fn chunk_text_splits_large_input() {
        let input = format!("{}\n\n{}", "a".repeat(1300), "b".repeat(20));
        let chunks = chunk_text(&input);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn detect_room_uses_path_and_keyword_rules() {
        let root = Path::new("/tmp/project");
        let path = Path::new("/tmp/project/src/security.txt");
        let rooms = vec![
            crate::room_detector::ProjectRoom {
                name: "auth".to_string(),
                keywords: vec!["jwt".to_string(), "token".to_string()],
            },
            crate::room_detector::ProjectRoom {
                name: "docs".to_string(),
                keywords: vec!["guide".to_string()],
            },
        ];
        assert_eq!(
            detect_room(root, path, "JWT token handling and auth flows", &rooms),
            "auth"
        );
    }

    #[test]
    fn normalize_search_hits_uses_python_style_similarity_and_basename() {
        let hits = normalize_search_hits(vec![
            SearchHit {
                id: "b".to_string(),
                text: "second".to_string(),
                wing: "project".to_string(),
                room: "auth".to_string(),
                source_file: "/tmp/project/src/auth.txt".to_string(),
                source_path: "/tmp/project/src/auth.txt".to_string(),
                source_mtime: Some(1.0),
                chunk_index: 1,
                added_by: Some("codex".to_string()),
                filed_at: Some("2026-04-13T00:00:00Z".to_string()),
                similarity: Some(0.81249),
                score: Some(0.18751),
            },
            SearchHit {
                id: "a".to_string(),
                text: "first".to_string(),
                wing: "project".to_string(),
                room: "auth".to_string(),
                source_file: "".to_string(),
                source_path: "/tmp/project/src/zeta.txt".to_string(),
                source_mtime: Some(1.0),
                chunk_index: 0,
                added_by: Some("codex".to_string()),
                filed_at: Some("2026-04-13T00:00:00Z".to_string()),
                similarity: Some(0.81251),
                score: Some(0.18749),
            },
        ]);

        assert_eq!(hits[0].source_file, "zeta.txt");
        assert_eq!(hits[0].similarity, Some(0.813));
        assert_eq!(hits[1].source_file, "auth.txt");
        assert_eq!(hits[1].similarity, Some(0.812));
    }

    #[test]
    fn normalize_search_hits_keeps_duplicate_files_as_separate_hits() {
        let hits = normalize_search_hits(vec![
            SearchHit {
                id: "chunk-2".to_string(),
                text: "later".to_string(),
                wing: "project".to_string(),
                room: "auth".to_string(),
                source_file: "auth.txt".to_string(),
                source_path: "/tmp/project/src/auth.txt".to_string(),
                source_mtime: None,
                chunk_index: 2,
                added_by: None,
                filed_at: None,
                similarity: Some(0.7),
                score: Some(0.3),
            },
            SearchHit {
                id: "chunk-1".to_string(),
                text: "earlier".to_string(),
                wing: "project".to_string(),
                room: "auth".to_string(),
                source_file: "auth.txt".to_string(),
                source_path: "/tmp/project/src/auth.txt".to_string(),
                source_mtime: None,
                chunk_index: 1,
                added_by: None,
                filed_at: None,
                similarity: Some(0.7),
                score: Some(0.3),
            },
        ]);

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "chunk-1");
        assert_eq!(hits[1].id, "chunk-2");
    }
}
