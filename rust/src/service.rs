use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::VERSION;
use crate::bootstrap::bootstrap_project;
use crate::compress::{CompressSummaryContext, CompressionRun};
use crate::config::AppConfig;
use crate::dedup::{DedupSummaryContext, Deduplicator};
use crate::dialect::{Dialect, count_tokens};
use crate::drawers::{build_manual_drawer, drawer_input_from_record, sanitize_name};
use crate::embed::{EmbeddingProvider, build_embedder};
use crate::embedding_runtime::{
    EmbeddingRuntimeContext, finalize_doctor_summary, prepare_embedding_run,
};
use crate::entity_detector::detect_entities_for_registry;
use crate::error::Result;
use crate::knowledge_graph::KnowledgeGraph;
use crate::layers::{read_identity_text, render_layer1, render_layer2};
use crate::miner::mine_project_run;
use crate::model::{
    CompressSummary, DedupSummary, DiaryReadResult, DiaryWriteResult, DoctorSummary,
    DrawerDeleteResult, DrawerWriteResult, GraphStats, GraphTraversalResult, InitSummary,
    KgInvalidateResult, KgQueryResult, KgStats, KgTimelineResult, KgTriple, KgWriteResult,
    LayerStatusSummary, MigrateSummary, MineProgressEvent, MineRequest, MineSummary,
    PrepareEmbeddingSummary, RecallSummary, RegistryConfirmResult, RegistryLearnResult,
    RegistryLookupResult, RegistryQueryResult, RegistryResearchResult, RegistrySummaryResult,
    RegistryWriteResult, RepairPruneSummary, RepairRebuildSummary, RepairScanSummary,
    RepairSummary, Rooms, SearchFilters, SearchHit, SearchResults, Status, Taxonomy, TunnelRoom,
    WakeUpSummary,
};
use crate::palace::ensure_vector_store;
use crate::palace_graph::{
    build_room_graph, find_tunnels as find_graph_tunnels, graph_stats as summarize_graph,
    traverse_graph as traverse_room_graph,
};
use crate::registry::EntityRegistry;
use crate::repair::{RepairContext, RepairDiagnostics, backup_sqlite_source, read_corrupt_ids};
use crate::searcher::{normalize_search_hits, normalize_source_file};
use crate::storage::sqlite::{CURRENT_SCHEMA_VERSION, SqliteStore};
use crate::storage::vector::VectorStore;

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
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let _vector = ensure_vector_store(&self.config, self.embedder.profile()).await?;
        let schema_version = sqlite.schema_version()?.unwrap_or(CURRENT_SCHEMA_VERSION);

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
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let _vector = ensure_vector_store(&self.config, self.embedder.profile()).await?;
        let schema_version = sqlite.schema_version()?.unwrap_or(CURRENT_SCHEMA_VERSION);
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

    pub async fn status(&self) -> Result<Status> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        Ok(Status {
            kind: "status".to_string(),
            total_drawers: sqlite.total_drawers()?,
            wings: sqlite.list_wings()?,
            rooms: sqlite.list_rooms(None)?.rooms,
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            schema_version: sqlite.schema_version()?.unwrap_or(CURRENT_SCHEMA_VERSION),
        })
    }

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

    pub async fn repair(&self) -> Result<RepairSummary> {
        let context = RepairContext {
            palace_path: self.config.palace_path.clone(),
            sqlite_path: self.config.sqlite_path(),
            lance_path: self.config.lance_path(),
            version: VERSION.to_string(),
        };
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

    pub async fn repair_scan(&self, wing: Option<&str>) -> Result<RepairScanSummary> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        let sqlite_drawers = sqlite.list_drawers(wing)?;
        let vector_drawers = vector
            .list_drawers(self.embedder.profile().dimension, wing, None)
            .await?;

        let context = RepairContext {
            palace_path: self.config.palace_path.clone(),
            sqlite_path: self.config.sqlite_path(),
            lance_path: self.config.lance_path(),
            version: VERSION.to_string(),
        };
        Ok(context.build_scan_summary(wing, &sqlite_drawers, &vector_drawers)?)
    }

    pub async fn repair_prune(&self, confirm: bool) -> Result<RepairPruneSummary> {
        self.config.ensure_dirs()?;
        let context = RepairContext {
            palace_path: self.config.palace_path.clone(),
            sqlite_path: self.config.sqlite_path(),
            lance_path: self.config.lance_path(),
            version: VERSION.to_string(),
        };
        let queued_ids = read_corrupt_ids(&context.corrupt_ids_path())?;

        if !confirm {
            return Ok(context.build_prune_preview(&queued_ids, confirm));
        }

        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;

        let deleted_from_sqlite = sqlite.delete_drawers(&queued_ids)?;
        let deleted_from_vector = vector
            .delete_drawers(self.embedder.profile().dimension, &queued_ids)
            .await?;

        Ok(context.build_prune_result(
            &queued_ids,
            confirm,
            deleted_from_vector,
            deleted_from_sqlite,
            0,
        ))
    }

    pub async fn repair_rebuild(&self) -> Result<RepairRebuildSummary> {
        self.config.ensure_dirs()?;
        let sqlite_path = self.config.sqlite_path();
        let sqlite = SqliteStore::open(&sqlite_path)?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let drawers = sqlite.list_drawers(None)?;

        let backup_path = backup_sqlite_source(&sqlite_path)?;

        let vector = VectorStore::connect(&self.config.lance_path()).await?;
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

        let context = RepairContext {
            palace_path: self.config.palace_path.clone(),
            sqlite_path: self.config.sqlite_path(),
            lance_path: self.config.lance_path(),
            version: VERSION.to_string(),
        };
        Ok(context.build_rebuild_summary(drawers.len(), rebuilt, backup_path))
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
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
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

    pub async fn doctor(&self, warm_embedding: bool) -> Result<DoctorSummary> {
        self.config.ensure_dirs()?;
        let context = EmbeddingRuntimeContext {
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            provider: self.embedder.profile().provider.clone(),
            model: self.embedder.profile().model.clone(),
        };
        let summary = self.embedder.doctor(&context.palace_path, warm_embedding);
        Ok(finalize_doctor_summary(summary, &context))
    }

    pub async fn prepare_embedding(
        &self,
        attempts: usize,
        wait_ms: u64,
    ) -> Result<PrepareEmbeddingSummary> {
        self.config.ensure_dirs()?;
        let context = EmbeddingRuntimeContext {
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            provider: self.embedder.profile().provider.clone(),
            model: self.embedder.profile().model.clone(),
        };
        let run = prepare_embedding_run(
            self.embedder.clone(),
            &context.palace_path,
            attempts,
            wait_ms,
        )
        .await?;
        Ok(run.into_summary(&context))
    }

    pub async fn list_wings(&self) -> Result<BTreeMap<String, usize>> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        sqlite.list_wings()
    }

    pub async fn list_rooms(&self, wing: Option<&str>) -> Result<Rooms> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        sqlite.list_rooms(wing)
    }

    pub async fn taxonomy(&self) -> Result<Taxonomy> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        sqlite.taxonomy()
    }

    pub async fn traverse_graph(
        &self,
        start_room: &str,
        max_hops: usize,
    ) -> Result<GraphTraversalResult> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let graph = build_room_graph(&sqlite.graph_room_rows()?);
        Ok(traverse_room_graph(&graph, start_room, max_hops))
    }

    pub async fn find_tunnels(
        &self,
        wing_a: Option<&str>,
        wing_b: Option<&str>,
    ) -> Result<Vec<TunnelRoom>> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let graph = build_room_graph(&sqlite.graph_room_rows()?);
        Ok(find_graph_tunnels(&graph, wing_a, wing_b))
    }

    pub async fn graph_stats(&self) -> Result<GraphStats> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let graph = build_room_graph(&sqlite.graph_room_rows()?);
        Ok(summarize_graph(&graph))
    }

    pub async fn search(
        &self,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Result<SearchResults> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        let embedding = self.embedder.embed_query(query)?;
        let hits = normalize_search_hits(vector.search(&embedding, wing, room, limit).await?);
        Ok(SearchResults {
            query: query.to_string(),
            filters: SearchFilters {
                wing: wing.map(ToOwned::to_owned),
                room: room.map(ToOwned::to_owned),
            },
            results: hits,
        })
    }

    pub async fn compress(&self, wing: Option<&str>, dry_run: bool) -> Result<CompressSummary> {
        self.config.ensure_dirs()?;
        let dialect = Dialect;
        let mut sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let drawers = sqlite.list_drawers(wing)?;
        let run = CompressionRun::from_drawers(drawers, &dialect);

        if !dry_run {
            sqlite.replace_compressed_drawers(wing, &run.entries)?;
        }

        Ok(run.into_summary(CompressSummaryContext {
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            wing: wing.map(ToOwned::to_owned),
            dry_run,
        }))
    }

    pub async fn wake_up(&self, wing: Option<&str>) -> Result<WakeUpSummary> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let identity_path = self.config.identity_path();
        let identity = read_identity_text(&identity_path);

        let recent = sqlite.recent_drawers(wing, 15)?;
        let layer1 = render_layer1(&recent, wing);
        let token_estimate = count_tokens(&identity) + count_tokens(&layer1);

        Ok(WakeUpSummary {
            kind: "wake_up".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            wing: wing.map(ToOwned::to_owned),
            identity_path: identity_path.display().to_string(),
            identity,
            layer1,
            token_estimate,
        })
    }

    pub async fn recall(
        &self,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> Result<RecallSummary> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let matches = sqlite.list_drawers(wing)?;
        let mut hits = matches
            .into_iter()
            .filter(|record| room.map(|value| value == record.room).unwrap_or(true))
            .map(|record| SearchHit {
                id: record.id,
                text: record.text,
                wing: record.wing,
                room: record.room,
                source_file: normalize_source_file(&record.source_file, &record.source_path),
                source_path: record.source_path,
                source_mtime: record.source_mtime,
                chunk_index: record.chunk_index,
                added_by: Some(record.added_by),
                filed_at: Some(record.filed_at),
                similarity: None,
                score: None,
            })
            .collect::<Vec<_>>();

        hits.sort_by(|left, right| {
            left.wing
                .cmp(&right.wing)
                .then_with(|| left.room.cmp(&right.room))
                .then_with(|| left.source_file.cmp(&right.source_file))
                .then_with(|| left.chunk_index.cmp(&right.chunk_index))
        });

        let total_matches = hits.len();
        let n_results = n_results.max(1);
        hits.truncate(n_results);
        let text = render_layer2(&hits, wing, room);

        Ok(RecallSummary {
            kind: "recall".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            wing: wing.map(ToOwned::to_owned),
            room: room.map(ToOwned::to_owned),
            n_results,
            total_matches,
            text,
            results: hits,
        })
    }

    pub async fn layer_status(&self) -> Result<LayerStatusSummary> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let identity_path = self.config.identity_path();
        let identity_exists = identity_path.exists();
        let identity_text = if identity_exists {
            fs::read_to_string(&identity_path)
                .map(|text| text.trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        Ok(LayerStatusSummary {
            kind: "layers_status".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            identity_path: identity_path.display().to_string(),
            identity_exists,
            identity_tokens: count_tokens(&identity_text),
            total_drawers: sqlite.total_drawers()?,
            layer0_description: "Identity text loaded from palace-local identity.txt".to_string(),
            layer1_description: "Essential story auto-generated from recent drawers".to_string(),
            layer2_description: "On-demand wing/room recall from stored drawers".to_string(),
            layer3_description: "Deep semantic search across the whole palace".to_string(),
        })
    }

    pub fn registry_summary(&self, project_dir: &Path) -> Result<RegistrySummaryResult> {
        let registry_path = project_dir.join("entity_registry.json");
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

    pub fn registry_lookup(
        &self,
        project_dir: &Path,
        word: &str,
        context: &str,
    ) -> Result<RegistryLookupResult> {
        let registry_path = project_dir.join("entity_registry.json");
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

    pub fn registry_learn(&self, project_dir: &Path) -> Result<RegistryLearnResult> {
        let registry_path = project_dir.join("entity_registry.json");
        let mut registry = EntityRegistry::load(&registry_path)?;
        let (people, projects) = detect_entities_for_registry(project_dir)?;
        let learned = registry.learn(&people, &projects);
        registry.save(&registry_path)?;
        Ok(RegistryLearnResult {
            kind: "registry_learn".to_string(),
            project_path: project_dir.display().to_string(),
            registry_path: registry_path.display().to_string(),
            added_people: learned.added_people,
            added_projects: learned.added_projects,
            total_people: learned.total_people,
            total_projects: learned.total_projects,
        })
    }

    pub fn registry_add_person(
        &self,
        project_dir: &Path,
        name: &str,
        relationship: &str,
        context: &str,
    ) -> Result<RegistryWriteResult> {
        let registry_path = project_dir.join("entity_registry.json");
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

    pub fn registry_add_project(
        &self,
        project_dir: &Path,
        project: &str,
    ) -> Result<RegistryWriteResult> {
        let registry_path = project_dir.join("entity_registry.json");
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

    pub fn registry_add_alias(
        &self,
        project_dir: &Path,
        canonical: &str,
        alias: &str,
    ) -> Result<RegistryWriteResult> {
        let registry_path = project_dir.join("entity_registry.json");
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

    pub fn registry_query(&self, project_dir: &Path, query: &str) -> Result<RegistryQueryResult> {
        let registry_path = project_dir.join("entity_registry.json");
        let registry = EntityRegistry::load(&registry_path)?;
        Ok(RegistryQueryResult {
            kind: "registry_query".to_string(),
            registry_path: registry_path.display().to_string(),
            query: query.to_string(),
            people: registry.extract_people_from_query(query),
            unknown_candidates: registry.extract_unknown_candidates(query),
        })
    }

    pub fn registry_research(
        &self,
        project_dir: &Path,
        word: &str,
        auto_confirm: bool,
    ) -> Result<RegistryResearchResult> {
        let registry_path = project_dir.join("entity_registry.json");
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

    pub fn registry_confirm_research(
        &self,
        project_dir: &Path,
        word: &str,
        entity_type: &str,
        relationship: &str,
        context: &str,
    ) -> Result<RegistryConfirmResult> {
        let registry_path = project_dir.join("entity_registry.json");
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
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        KnowledgeGraph::new(&sqlite).add_triple(triple).map(|_| ())
    }

    pub async fn query_kg(&self, subject: &str) -> Result<Vec<KgTriple>> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        KnowledgeGraph::new(&sqlite).query_raw(subject)
    }

    pub async fn kg_query(
        &self,
        entity: &str,
        as_of: Option<&str>,
        direction: &str,
    ) -> Result<KgQueryResult> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        KnowledgeGraph::new(&sqlite).query_entity(entity, as_of, direction)
    }

    pub async fn kg_timeline(&self, entity: Option<&str>) -> Result<KgTimelineResult> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        KnowledgeGraph::new(&sqlite).timeline(entity)
    }

    pub async fn kg_stats(&self) -> Result<KgStats> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        KnowledgeGraph::new(&sqlite).stats()
    }

    pub async fn kg_add(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        valid_from: Option<&str>,
    ) -> Result<KgWriteResult> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        KnowledgeGraph::new(&sqlite).add_triple(&KgTriple {
            subject: sanitize_name(subject, "subject")?,
            predicate: sanitize_name(predicate, "predicate")?,
            object: sanitize_name(object, "object")?,
            valid_from: valid_from.map(ToOwned::to_owned),
            valid_to: None,
        })
    }

    pub async fn kg_invalidate(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        ended: Option<&str>,
    ) -> Result<KgInvalidateResult> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        KnowledgeGraph::new(&sqlite).invalidate(
            &sanitize_name(subject, "subject")?,
            &sanitize_name(predicate, "predicate")?,
            &sanitize_name(object, "object")?,
            ended,
        )
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
        let drawer = build_manual_drawer(wing, room, content, source_file, added_by)?;

        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        let sqlite_result = sqlite.insert_drawer(&drawer)?;
        if sqlite_result.reason.as_deref() == Some("already_exists") {
            return Ok(sqlite_result);
        }

        let embedding = self.embedder.embed_query(&drawer.text)?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        vector.add_drawers(&[drawer], &[embedding]).await?;
        Ok(sqlite_result)
    }

    pub async fn delete_drawer(&self, drawer_id: &str) -> Result<DrawerDeleteResult> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        let result = sqlite.delete_drawer(drawer_id)?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        vector
            .delete_drawer(self.embedder.profile().dimension, drawer_id)
            .await?;
        Ok(result)
    }

    pub async fn diary_write(
        &self,
        agent_name: &str,
        entry: &str,
        topic: &str,
    ) -> Result<DiaryWriteResult> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.add_diary_entry(agent_name, topic, entry)
    }

    pub async fn diary_read(&self, agent_name: &str, last_n: usize) -> Result<DiaryReadResult> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.read_diary_entries(agent_name, last_n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::miner::chunk_text;
    use crate::room_detector::detect_room;

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
