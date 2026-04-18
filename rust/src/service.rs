use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::VERSION;
use crate::bootstrap::bootstrap_project;
use crate::config::AppConfig;
use crate::convo::{
    ConversationChunk, MIN_CONVO_CHUNK_SIZE, detect_convo_room, exchange_rooms,
    extract_exchange_chunks, extract_general_memories, general_rooms, normalize_conversation_file,
    scan_convo_files,
};
use crate::dialect::{CompressMetadata, Dialect, count_tokens};
use crate::embed::{EmbeddingProvider, build_embedder};
use crate::entity_detector::detect_entities_for_registry;
use crate::error::{MempalaceError, Result};
use crate::model::{
    CompressSummary, DedupSourceResult, DedupSummary, DiaryReadResult, DiaryWriteResult,
    DoctorSummary, DrawerDeleteResult, DrawerInput, DrawerWriteResult, GraphStats,
    GraphStatsTunnel, GraphTraversalError, GraphTraversalNode, GraphTraversalResult, InitSummary,
    KgInvalidateResult, KgQueryResult, KgStats, KgTimelineResult, KgTriple, KgWriteResult,
    LayerStatusSummary, MigrateSummary, MineProgressEvent, MineRequest, MineSummary,
    PrepareEmbeddingSummary, RecallSummary, RegistryConfirmResult, RegistryLearnResult,
    RegistryLookupResult, RegistryQueryResult, RegistryResearchResult, RegistrySummaryResult,
    RegistryWriteResult, RepairPruneSummary, RepairRebuildSummary, RepairScanSummary,
    RepairSummary, Rooms, SearchFilters, SearchHit, SearchResults, Status, Taxonomy, TunnelRoom,
    WakeUpSummary,
};
use crate::palace::{SKIP_DIRS, ensure_vector_store, source_state_matches};
use crate::registry::EntityRegistry;
use crate::room_detector::{detect_room, load_project_config, load_project_rooms};
use crate::storage::sqlite::{CURRENT_SCHEMA_VERSION, DrawerRecord, GraphRoomRow, SqliteStore};
use crate::storage::vector::VectorStore;
use chrono::Utc;
use ignore::WalkBuilder;

const READABLE_EXTENSIONS: &[&str] = &[
    ".txt", ".md", ".py", ".js", ".ts", ".jsx", ".tsx", ".json", ".yaml", ".yml", ".html", ".css",
    ".java", ".go", ".rs", ".rb", ".sh", ".csv", ".sql", ".toml",
];

const SKIP_FILENAMES: &[&str] = &[
    "mempalace.yaml",
    "mempalace.yml",
    "mempal.yaml",
    "mempal.yml",
    "entities.json",
    "entity_registry.json",
    "aaak_entities.md",
    "critical_facts.md",
    ".gitignore",
    "package-lock.json",
];

const CHUNK_SIZE: usize = 800;
const CHUNK_OVERLAP: usize = 100;
const MIN_CHUNK_SIZE: usize = 50;
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

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
        let palace_path = self.config.palace_path.display().to_string();
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

        Ok(RepairSummary {
            kind: "repair".to_string(),
            palace_path,
            sqlite_path: sqlite_path.display().to_string(),
            lance_path: lance_path.display().to_string(),
            version: VERSION.to_string(),
            sqlite_exists,
            lance_exists,
            schema_version,
            sqlite_drawer_count,
            embedding_provider,
            embedding_model,
            embedding_dimension,
            vector_accessible,
            ok: issues.is_empty(),
            issues,
        })
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

        let sqlite_ids = sqlite_drawers
            .iter()
            .map(|drawer| drawer.id.clone())
            .collect::<BTreeSet<_>>();
        let vector_ids = vector_drawers
            .iter()
            .map(|drawer| drawer.id.clone())
            .collect::<BTreeSet<_>>();

        let missing_from_vector = sqlite_ids
            .difference(&vector_ids)
            .cloned()
            .collect::<Vec<_>>();
        let orphaned_in_vector = vector_ids
            .difference(&sqlite_ids)
            .cloned()
            .collect::<Vec<_>>();
        let prune_candidates = orphaned_in_vector.len();

        let corrupt_ids_path = self.config.palace_path.join("corrupt_ids.txt");
        let mut payload = String::new();
        for drawer_id in &orphaned_in_vector {
            payload.push_str(drawer_id);
            payload.push('\n');
        }
        fs::write(&corrupt_ids_path, payload)?;

        Ok(RepairScanSummary {
            kind: "repair_scan".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            wing: wing.map(ToOwned::to_owned),
            sqlite_drawers: sqlite_drawers.len(),
            vector_drawers: vector_drawers.len(),
            missing_from_vector,
            orphaned_in_vector,
            corrupt_ids_path: corrupt_ids_path.display().to_string(),
            prune_candidates,
        })
    }

    pub async fn repair_prune(&self, confirm: bool) -> Result<RepairPruneSummary> {
        self.config.ensure_dirs()?;
        let corrupt_ids_path = self.config.palace_path.join("corrupt_ids.txt");
        let queued_ids = if corrupt_ids_path.exists() {
            fs::read_to_string(&corrupt_ids_path)?
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if !confirm {
            return Ok(RepairPruneSummary {
                kind: "repair_prune".to_string(),
                palace_path: self.config.palace_path.display().to_string(),
                sqlite_path: self.config.sqlite_path().display().to_string(),
                lance_path: self.config.lance_path().display().to_string(),
                version: VERSION.to_string(),
                corrupt_ids_path: corrupt_ids_path.display().to_string(),
                queued: queued_ids.len(),
                confirm,
                deleted_from_vector: 0,
                deleted_from_sqlite: 0,
                failed: 0,
            });
        }

        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;

        let deleted_from_sqlite = sqlite.delete_drawers(&queued_ids)?;
        let deleted_from_vector = vector
            .delete_drawers(self.embedder.profile().dimension, &queued_ids)
            .await?;

        Ok(RepairPruneSummary {
            kind: "repair_prune".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            corrupt_ids_path: corrupt_ids_path.display().to_string(),
            queued: queued_ids.len(),
            confirm,
            deleted_from_vector,
            deleted_from_sqlite,
            failed: 0,
        })
    }

    pub async fn repair_rebuild(&self) -> Result<RepairRebuildSummary> {
        self.config.ensure_dirs()?;
        let sqlite_path = self.config.sqlite_path();
        let sqlite = SqliteStore::open(&sqlite_path)?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let drawers = sqlite.list_drawers(None)?;

        let backup_path = if sqlite_path.exists() {
            let backup_path = sqlite_path.with_extension("sqlite3.backup");
            fs::copy(&sqlite_path, &backup_path)?;
            Some(backup_path.display().to_string())
        } else {
            None
        };

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

        Ok(RepairRebuildSummary {
            kind: "repair_rebuild".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            drawers_found: drawers.len(),
            rebuilt,
            backup_path,
        })
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
        let vectors_by_id = vector_drawers
            .into_iter()
            .map(|drawer| (drawer.id.clone(), drawer))
            .collect::<HashMap<_, _>>();

        let mut grouped = BTreeMap::<String, Vec<DrawerRecord>>::new();
        for drawer in sqlite_drawers {
            if let Some(pattern) = source_pattern
                && !drawer
                    .source_file
                    .to_ascii_lowercase()
                    .contains(&pattern.to_ascii_lowercase())
            {
                continue;
            }
            grouped
                .entry(drawer.source_file.clone())
                .or_default()
                .push(drawer);
        }

        let mut groups = Vec::new();
        let mut delete_ids = Vec::new();
        let mut kept = 0usize;
        let mut total_drawers = 0usize;

        for (source_file, mut records) in grouped {
            if records.len() < min_count {
                continue;
            }
            total_drawers += records.len();
            records.sort_by(|left, right| right.text.len().cmp(&left.text.len()));

            let mut kept_vectors = Vec::<(&str, &Vec<f32>)>::new();
            let mut local_deleted = 0usize;
            let mut local_kept = 0usize;

            for record in &records {
                let Some(vector_record) = vectors_by_id.get(&record.id) else {
                    continue;
                };
                let is_dup = kept_vectors.iter().any(|(_, kept_vector)| {
                    cosine_distance(&vector_record.vector, kept_vector) < threshold
                });
                if is_dup {
                    delete_ids.push(record.id.clone());
                    local_deleted += 1;
                } else {
                    kept_vectors.push((record.id.as_str(), &vector_record.vector));
                    local_kept += 1;
                }
            }

            kept += local_kept;
            groups.push(DedupSourceResult {
                source_file,
                before: records.len(),
                kept: local_kept,
                deleted: local_deleted,
            });
        }

        groups.sort_by(|left, right| {
            right
                .before
                .cmp(&left.before)
                .then(left.source_file.cmp(&right.source_file))
        });

        if !dry_run && !stats_only && !delete_ids.is_empty() {
            sqlite.delete_drawers(&delete_ids)?;
            vector
                .delete_drawers(self.embedder.profile().dimension, &delete_ids)
                .await?;
        }

        Ok(DedupSummary {
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
            sources_checked: groups.len(),
            total_drawers,
            kept,
            deleted: delete_ids.len(),
            stats_only,
            groups,
        })
    }

    pub async fn doctor(&self, warm_embedding: bool) -> Result<DoctorSummary> {
        self.config.ensure_dirs()?;
        let mut summary = self.embedder.doctor(
            &self.config.palace_path.display().to_string(),
            warm_embedding,
        );
        summary.sqlite_path = self.config.sqlite_path().display().to_string();
        summary.lance_path = self.config.lance_path().display().to_string();
        summary.version = VERSION.to_string();
        Ok(summary)
    }

    pub async fn prepare_embedding(
        &self,
        attempts: usize,
        wait_ms: u64,
    ) -> Result<PrepareEmbeddingSummary> {
        self.config.ensure_dirs()?;

        let total_attempts = attempts.max(1);
        let mut last_error = None;
        let mut last_doctor = self
            .embedder
            .doctor(&self.config.palace_path.display().to_string(), false);

        for attempt in 0..total_attempts {
            let doctor = self
                .embedder
                .doctor(&self.config.palace_path.display().to_string(), true);
            let success = doctor.warmup_ok;
            last_error = doctor.warmup_error.clone();
            last_doctor = doctor;

            if success {
                return Ok(PrepareEmbeddingSummary {
                    kind: "prepare_embedding".to_string(),
                    palace_path: self.config.palace_path.display().to_string(),
                    sqlite_path: self.config.sqlite_path().display().to_string(),
                    lance_path: self.config.lance_path().display().to_string(),
                    version: VERSION.to_string(),
                    provider: self.embedder.profile().provider.clone(),
                    model: self.embedder.profile().model.clone(),
                    attempts: attempt + 1,
                    success: true,
                    last_error: None,
                    doctor: last_doctor,
                });
            }

            if attempt + 1 < total_attempts && wait_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
            }
        }

        Ok(PrepareEmbeddingSummary {
            kind: "prepare_embedding".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            provider: self.embedder.profile().provider.clone(),
            model: self.embedder.profile().model.clone(),
            attempts: total_attempts,
            success: false,
            last_error,
            doctor: last_doctor,
        })
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

        let Some(start) = graph.nodes.get(start_room) else {
            return Ok(GraphTraversalResult::Error(GraphTraversalError {
                error: format!("Room '{start_room}' not found"),
                suggestions: fuzzy_match_room(start_room, &graph.nodes),
            }));
        };

        let mut visited = BTreeSet::new();
        visited.insert(start_room.to_string());
        let mut results = vec![GraphTraversalNode {
            room: start_room.to_string(),
            wings: start.wings.iter().cloned().collect(),
            halls: start.halls.iter().cloned().collect(),
            count: start.count,
            hop: 0,
            connected_via: None,
        }];

        let mut frontier = vec![(start_room.to_string(), 0usize)];
        while let Some((current_room, depth)) = frontier.first().cloned() {
            frontier.remove(0);
            if depth >= max_hops {
                continue;
            }
            let current = match graph.nodes.get(&current_room) {
                Some(current) => current,
                None => continue,
            };
            for (room, data) in &graph.nodes {
                if visited.contains(room) {
                    continue;
                }
                let shared_wings = current
                    .wings
                    .intersection(&data.wings)
                    .cloned()
                    .collect::<Vec<_>>();
                if shared_wings.is_empty() {
                    continue;
                }
                visited.insert(room.clone());
                results.push(GraphTraversalNode {
                    room: room.clone(),
                    wings: data.wings.iter().cloned().collect(),
                    halls: data.halls.iter().cloned().collect(),
                    count: data.count,
                    hop: depth + 1,
                    connected_via: Some(shared_wings),
                });
                if depth + 1 < max_hops {
                    frontier.push((room.clone(), depth + 1));
                }
            }
        }

        results.sort_by(|left, right| {
            left.hop
                .cmp(&right.hop)
                .then(right.count.cmp(&left.count))
                .then(left.room.cmp(&right.room))
        });
        results.truncate(50);
        Ok(GraphTraversalResult::Results(results))
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

        let mut tunnels = graph
            .nodes
            .into_iter()
            .filter_map(|(room, data)| {
                if data.wings.len() < 2 {
                    return None;
                }
                if let Some(wing) = wing_a
                    && !data.wings.contains(wing)
                {
                    return None;
                }
                if let Some(wing) = wing_b
                    && !data.wings.contains(wing)
                {
                    return None;
                }
                Some(TunnelRoom {
                    room,
                    wings: data.wings.into_iter().collect(),
                    halls: data.halls.into_iter().collect(),
                    count: data.count,
                    recent: data.recent.unwrap_or_default(),
                })
            })
            .collect::<Vec<_>>();

        tunnels.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then(left.room.cmp(&right.room))
        });
        tunnels.truncate(50);
        Ok(tunnels)
    }

    pub async fn graph_stats(&self) -> Result<GraphStats> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let graph = build_room_graph(&sqlite.graph_room_rows()?);

        let tunnel_rooms = graph
            .nodes
            .values()
            .filter(|node| node.wings.len() >= 2)
            .count();

        let mut rooms_per_wing = BTreeMap::new();
        for node in graph.nodes.values() {
            for wing in &node.wings {
                *rooms_per_wing.entry(wing.clone()).or_insert(0) += 1;
            }
        }

        let mut top_tunnels = graph
            .nodes
            .iter()
            .filter(|(_, data)| data.wings.len() >= 2)
            .map(|(room, data)| GraphStatsTunnel {
                room: room.clone(),
                wings: data.wings.iter().cloned().collect(),
                count: data.count,
            })
            .collect::<Vec<_>>();
        top_tunnels.sort_by(|left, right| {
            right
                .wings
                .len()
                .cmp(&left.wings.len())
                .then(right.count.cmp(&left.count))
                .then(left.room.cmp(&right.room))
        });
        top_tunnels.truncate(10);

        Ok(GraphStats {
            total_rooms: graph.nodes.len(),
            tunnel_rooms,
            total_edges: graph.total_edges,
            rooms_per_wing,
            top_tunnels,
        })
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

        let mut original_tokens = 0usize;
        let mut compressed_tokens = 0usize;
        let entries = drawers
            .into_iter()
            .map(|drawer| {
                let aaak = dialect.compress(
                    &drawer.text,
                    CompressMetadata {
                        wing: &drawer.wing,
                        room: &drawer.room,
                        source_file: &drawer.source_file,
                        filed_at: Some(&drawer.filed_at),
                    },
                );
                let stats = dialect.compression_stats(&drawer.text, &aaak);
                original_tokens += stats.original_tokens;
                compressed_tokens += stats.compressed_tokens;
                crate::model::CompressedDrawer {
                    drawer_id: drawer.id,
                    wing: drawer.wing,
                    room: drawer.room,
                    source_file: drawer.source_file,
                    source_path: drawer.source_path,
                    ingest_mode: drawer.ingest_mode,
                    extract_mode: drawer.extract_mode,
                    aaak,
                    original_tokens: stats.original_tokens,
                    compressed_tokens: stats.compressed_tokens,
                    compression_ratio: stats.ratio,
                }
            })
            .collect::<Vec<_>>();

        if !dry_run {
            sqlite.replace_compressed_drawers(wing, &entries)?;
        }

        Ok(CompressSummary {
            kind: "compress".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            wing: wing.map(ToOwned::to_owned),
            dry_run,
            processed: entries.len(),
            stored: if dry_run { 0 } else { entries.len() },
            original_tokens,
            compressed_tokens,
            compression_ratio: original_tokens as f64 / compressed_tokens.max(1) as f64,
            entries,
        })
    }

    pub async fn wake_up(&self, wing: Option<&str>) -> Result<WakeUpSummary> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        let identity_path = self.config.identity_path();
        let identity = if identity_path.exists() {
            fs::read_to_string(&identity_path)
                .map(|text| text.trim().to_string())
                .unwrap_or_else(|_| default_identity_text())
        } else {
            default_identity_text()
        };

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
        mut on_progress: F,
    ) -> Result<MineSummary>
    where
        F: FnMut(MineProgressEvent),
    {
        if request.mode == "convos" {
            return self
                .mine_conversations_with_progress(dir, request, on_progress)
                .await;
        }
        if request.mode != "projects" {
            return Err(MempalaceError::InvalidArgument(format!(
                "Unsupported mine mode: {}",
                request.mode
            )));
        }
        if !dir.exists() {
            return Err(MempalaceError::InvalidArgument(format!(
                "Project directory does not exist: {}",
                dir.display()
            )));
        }

        self.init().await?;
        let wing = request.wing.clone().unwrap_or_else(|| {
            load_project_config(dir)
                .ok()
                .flatten()
                .and_then(|config| config.wing)
                .or_else(|| {
                    dir.file_name()
                        .map(|name| name.to_string_lossy().to_string())
                })
                .unwrap_or_else(|| "project".to_string())
        });
        let rooms = load_project_rooms(dir)?;
        let configured_rooms = rooms
            .iter()
            .map(|room| room.name.clone())
            .collect::<Vec<_>>();

        let files = discover_files(dir, request.respect_gitignore, &request.include_ignored)?;
        let files_planned = if request.limit == 0 {
            files.len()
        } else {
            files.len().min(request.limit)
        };
        let vector = if request.dry_run {
            None
        } else {
            Some(VectorStore::connect(&self.config.lance_path()).await?)
        };
        let mut sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;

        let mut files_seen = 0_usize;
        let mut files_mined = 0_usize;
        let mut files_skipped_unchanged = 0_usize;
        let mut drawers_added = 0_usize;
        let mut room_counts = BTreeMap::new();
        let total = files_planned;

        for path in files.into_iter().take(if request.limit == 0 {
            usize::MAX
        } else {
            request.limit
        }) {
            files_seen += 1;
            let Some(contents) = read_text_file(&path)? else {
                continue;
            };

            let source_path_buf = path.canonicalize()?;
            let source_path = source_path_buf.display().to_string();
            let source_mtime = SqliteStore::source_mtime(&source_path_buf);
            let source_hash = blake3::hash(contents.as_bytes()).to_hex().to_string();
            if source_state_matches(&sqlite, &source_path_buf, &source_hash, source_mtime, true)? {
                files_skipped_unchanged += 1;
                continue;
            }

            let room = detect_room(dir, &path, &contents, &rooms);
            let chunks = chunk_text(&contents);
            if chunks.is_empty() {
                continue;
            }
            let source_file = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| source_path.clone());
            let filed_at = Utc::now().to_rfc3339();

            let drawers: Vec<DrawerInput> = chunks
                .iter()
                .enumerate()
                .map(|(idx, chunk)| DrawerInput {
                    id: format!(
                        "drawer_{}_{}_{}_{}",
                        sanitize_slug(&wing),
                        sanitize_slug(&room),
                        blake3::hash(source_path.as_bytes()).to_hex(),
                        idx
                    ),
                    wing: wing.clone(),
                    room: room.clone(),
                    source_file: source_file.clone(),
                    source_path: source_path.clone(),
                    source_hash: source_hash.clone(),
                    source_mtime,
                    chunk_index: idx as i32,
                    added_by: request.agent.clone(),
                    filed_at: filed_at.clone(),
                    ingest_mode: "projects".to_string(),
                    extract_mode: request.extract.clone(),
                    text: chunk.clone(),
                })
                .collect();

            drawers_added += drawers.len();
            files_mined += 1;
            *room_counts.entry(room.clone()).or_insert(0) += 1;

            if request.dry_run {
                on_progress(MineProgressEvent::DryRun {
                    file_name: source_file,
                    room,
                    drawers: drawers.len(),
                });
                continue;
            }

            let embeddings = self.embedder.embed_documents(&chunks)?;
            if let Some(vector) = &vector {
                vector.replace_source(&drawers, &embeddings).await?;
            }
            sqlite.replace_source(
                &source_path,
                &wing,
                &room,
                &source_hash,
                source_mtime,
                &drawers,
            )?;
            on_progress(MineProgressEvent::Filed {
                index: files_mined + files_skipped_unchanged,
                total,
                file_name: source_file,
                drawers: drawers.len(),
            });
        }

        Ok(MineSummary {
            kind: "mine".to_string(),
            mode: request.mode.clone(),
            extract: request.extract.clone(),
            agent: request.agent.clone(),
            wing,
            configured_rooms,
            project_path: dir.display().to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            version: VERSION.to_string(),
            dry_run: request.dry_run,
            filters: SearchFilters {
                wing: request.wing.clone(),
                room: None,
            },
            respect_gitignore: request.respect_gitignore,
            include_ignored: request.include_ignored.clone(),
            files_planned,
            files_seen,
            files_processed: files_mined,
            files_mined,
            drawers_added,
            files_skipped: files_skipped_unchanged,
            files_skipped_unchanged,
            room_counts,
            next_hint: "mempalace search \"what you're looking for\"".to_string(),
        })
    }

    async fn mine_conversations_with_progress<F>(
        &self,
        dir: &Path,
        request: &MineRequest,
        mut on_progress: F,
    ) -> Result<MineSummary>
    where
        F: FnMut(MineProgressEvent),
    {
        if !dir.exists() {
            return Err(MempalaceError::InvalidArgument(format!(
                "Conversation directory does not exist: {}",
                dir.display()
            )));
        }
        if !matches!(request.extract.as_str(), "exchange" | "general") {
            return Err(MempalaceError::InvalidArgument(format!(
                "Unsupported conversation extract mode: {}",
                request.extract
            )));
        }

        self.init().await?;
        let wing = request
            .wing
            .clone()
            .unwrap_or_else(|| default_convo_wing(dir));
        let configured_rooms = if request.extract == "general" {
            general_rooms()
        } else {
            exchange_rooms()
        };

        let files = scan_convo_files(
            dir,
            request.respect_gitignore,
            &request.include_ignored,
            SKIP_DIRS,
        )?;
        let files_planned = if request.limit == 0 {
            files.len()
        } else {
            files.len().min(request.limit)
        };
        let vector = if request.dry_run {
            None
        } else {
            Some(VectorStore::connect(&self.config.lance_path()).await?)
        };
        let mut sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;

        let mut files_seen = 0usize;
        let mut files_mined = 0usize;
        let mut files_skipped_unchanged = 0usize;
        let mut drawers_added = 0usize;
        let mut room_counts = BTreeMap::new();

        for path in files.into_iter().take(if request.limit == 0 {
            usize::MAX
        } else {
            request.limit
        }) {
            files_seen += 1;
            let source_path_buf = match path.canonicalize() {
                Ok(path) => path,
                Err(_) => continue,
            };
            let source_path = source_path_buf.display().to_string();
            let source_mtime = SqliteStore::source_mtime(&source_path_buf);
            let normalized = match normalize_conversation_file(&path) {
                Ok(Some(text)) => text,
                Ok(None) => continue,
                Err(_) => continue,
            };
            if normalized.trim().len() < MIN_CONVO_CHUNK_SIZE {
                continue;
            }

            let source_hash = blake3::hash(normalized.as_bytes()).to_hex().to_string();
            if source_state_matches(&sqlite, &source_path_buf, &source_hash, source_mtime, true)? {
                files_skipped_unchanged += 1;
                continue;
            }

            let chunks = if request.extract == "general" {
                extract_general_memories(&normalized, 0.3)
            } else {
                extract_exchange_chunks(&normalized)
            };
            if chunks.is_empty() {
                continue;
            }

            let source_file = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| source_path.clone());
            let filed_at = Utc::now().to_rfc3339();
            let drawers = build_conversation_drawers(
                &ConversationDrawerContext {
                    wing: &wing,
                    source_file: &source_file,
                    source_path: &source_path,
                    source_hash: &source_hash,
                    source_mtime,
                    agent: &request.agent,
                    filed_at: &filed_at,
                    extract_mode: &request.extract,
                },
                &chunks,
            );

            drawers_added += drawers.len();
            files_mined += 1;

            if request.extract == "general" {
                let mut per_file = BTreeMap::new();
                for chunk in &chunks {
                    *per_file.entry(chunk.room.clone()).or_insert(0usize) += 1;
                    *room_counts.entry(chunk.room.clone()).or_insert(0usize) += 1;
                }
                if request.dry_run {
                    let summary = per_file
                        .iter()
                        .map(|(room, count)| format!("{room}:{count}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    on_progress(MineProgressEvent::DryRunSummary {
                        file_name: source_file,
                        summary,
                        drawers: drawers.len(),
                    });
                    continue;
                }
            } else {
                let room = chunks
                    .first()
                    .map(|chunk| chunk.room.clone())
                    .unwrap_or_else(|| detect_convo_room(&normalized));
                *room_counts.entry(room.clone()).or_insert(0usize) += 1;
                if request.dry_run {
                    on_progress(MineProgressEvent::DryRun {
                        file_name: source_file,
                        room,
                        drawers: drawers.len(),
                    });
                    continue;
                }
            }

            let drawer_texts = drawers
                .iter()
                .map(|drawer| drawer.text.clone())
                .collect::<Vec<_>>();
            let embeddings = self.embedder.embed_documents(&drawer_texts)?;
            if let Some(vector) = &vector {
                vector.replace_source(&drawers, &embeddings).await?;
            }
            sqlite.replace_source(
                &source_path,
                &wing,
                chunks
                    .first()
                    .map(|chunk| chunk.room.as_str())
                    .unwrap_or("general"),
                &source_hash,
                source_mtime,
                &drawers,
            )?;
            on_progress(MineProgressEvent::Filed {
                index: files_mined + files_skipped_unchanged,
                total: files_planned,
                file_name: source_file,
                drawers: drawers.len(),
            });
        }

        Ok(MineSummary {
            kind: "mine".to_string(),
            mode: request.mode.clone(),
            extract: request.extract.clone(),
            agent: request.agent.clone(),
            wing,
            configured_rooms,
            project_path: dir.display().to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            version: VERSION.to_string(),
            dry_run: request.dry_run,
            filters: SearchFilters {
                wing: request.wing.clone(),
                room: None,
            },
            respect_gitignore: request.respect_gitignore,
            include_ignored: request.include_ignored.clone(),
            files_planned,
            files_seen,
            files_processed: files_mined,
            files_mined,
            drawers_added,
            files_skipped: files_seen.saturating_sub(files_mined),
            files_skipped_unchanged,
            room_counts,
            next_hint: "mempalace search \"what you're looking for\"".to_string(),
        })
    }

    pub async fn add_kg_triple(&self, triple: &KgTriple) -> Result<()> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.add_kg_triple(triple).map(|_| ())
    }

    pub async fn query_kg(&self, subject: &str) -> Result<Vec<KgTriple>> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.query_kg(subject)
    }

    pub async fn kg_query(
        &self,
        entity: &str,
        as_of: Option<&str>,
        direction: &str,
    ) -> Result<KgQueryResult> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        let facts = sqlite.query_kg_entity(entity, as_of, direction)?;
        Ok(KgQueryResult {
            entity: entity.to_string(),
            as_of: as_of.map(ToOwned::to_owned),
            count: facts.len(),
            facts,
        })
    }

    pub async fn kg_timeline(&self, entity: Option<&str>) -> Result<KgTimelineResult> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.kg_timeline(entity)
    }

    pub async fn kg_stats(&self) -> Result<KgStats> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.kg_stats()
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
        sqlite.add_kg_triple(&KgTriple {
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
        sqlite.invalidate_kg_triple(
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

        let sanitized_wing = sanitize_name(wing, "wing")?;
        let sanitized_room = sanitize_name(room, "room")?;
        let sanitized_content = sanitize_content(content)?;
        let sanitized_added_by = sanitize_name(added_by.unwrap_or("mcp"), "added_by")?;
        let normalized_source_file = source_file.unwrap_or_default().trim().to_string();
        let content_preview = sanitized_content
            .char_indices()
            .nth(100)
            .map(|(idx, _)| &sanitized_content[..idx])
            .unwrap_or(&sanitized_content);
        let wing_slug = identifier_fragment(&sanitized_wing);
        let room_slug = identifier_fragment(&sanitized_room);
        let drawer_id = format!(
            "drawer_{}_{}_{}",
            wing_slug,
            room_slug,
            &blake3::hash(
                format!("{sanitized_wing}|{sanitized_room}|{content_preview}").as_bytes()
            )
            .to_hex()
            .to_string()[..24]
        );
        let drawer = DrawerInput {
            id: drawer_id.clone(),
            wing: sanitized_wing.clone(),
            room: sanitized_room.clone(),
            source_file: normalized_source_file.clone(),
            source_path: if normalized_source_file.is_empty() {
                format!("mcp://{wing_slug}/{room_slug}/{drawer_id}")
            } else {
                normalized_source_file
            },
            source_hash: blake3::hash(sanitized_content.as_bytes())
                .to_hex()
                .to_string(),
            source_mtime: None,
            chunk_index: 0,
            added_by: sanitized_added_by,
            filed_at: Utc::now().to_rfc3339(),
            ingest_mode: "mcp".to_string(),
            extract_mode: "manual".to_string(),
            text: sanitized_content.clone(),
        };

        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        let sqlite_result = sqlite.insert_drawer(&drawer)?;
        if sqlite_result.reason.as_deref() == Some("already_exists") {
            return Ok(sqlite_result);
        }

        let embedding = self.embedder.embed_query(&sanitized_content)?;
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

fn discover_files(
    dir: &Path,
    respect_gitignore: bool,
    include_ignored: &[String],
) -> Result<Vec<PathBuf>> {
    let include_paths = normalize_include_paths(include_ignored);
    let include_paths_for_filter = include_paths.clone();
    let project_root = dir.to_path_buf();
    let mut builder = WalkBuilder::new(dir);
    builder.hidden(false);
    builder.git_ignore(respect_gitignore);
    builder.git_global(respect_gitignore);
    builder.git_exclude(respect_gitignore);
    builder.require_git(false);
    builder.filter_entry(move |entry| {
        if is_force_include(entry.path(), &project_root, &include_paths_for_filter) {
            return true;
        }

        entry
            .file_name()
            .to_str()
            .map(|name| !should_skip_dir(name))
            .unwrap_or(true)
    });

    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for result in builder.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.is_symlink() {
            continue;
        }

        let exact_force_include = is_exact_force_include(path, dir, &include_paths);
        if !exact_force_include && should_skip_file(path) {
            continue;
        }

        let stat = match path.metadata() {
            Ok(stat) => stat,
            Err(_) => continue,
        };
        if stat.len() > MAX_FILE_SIZE {
            continue;
        }

        if seen.insert(path.to_path_buf()) {
            files.push(path.to_path_buf());
        }
    }

    for rel in include_ignored {
        let path = dir.join(rel);
        if path.is_file() && seen.insert(path.clone()) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn read_text_file(path: &Path) -> Result<Option<String>> {
    let bytes = fs::read(path)?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}

fn chunk_text(text: &str) -> Vec<String> {
    let content = text.trim();
    if content.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < content.len() {
        let mut end = std::cmp::min(start + CHUNK_SIZE, content.len());
        if end < content.len() {
            if let Some(newline_pos) = content[start..end].rfind("\n\n") {
                let absolute = start + newline_pos;
                if absolute > start + CHUNK_SIZE / 2 {
                    end = absolute;
                } else if let Some(newline_pos) = content[start..end].rfind('\n') {
                    let absolute = start + newline_pos;
                    if absolute > start + CHUNK_SIZE / 2 {
                        end = absolute;
                    }
                }
            } else if let Some(newline_pos) = content[start..end].rfind('\n') {
                let absolute = start + newline_pos;
                if absolute > start + CHUNK_SIZE / 2 {
                    end = absolute;
                }
            }
        }

        let chunk = content[start..end].trim();
        if chunk.len() >= MIN_CHUNK_SIZE {
            chunks.push(chunk.to_string());
        }

        start = if end < content.len() {
            end.saturating_sub(CHUNK_OVERLAP)
        } else {
            end
        };
    }

    chunks
}

fn should_skip_dir(dirname: &str) -> bool {
    SKIP_DIRS.contains(&dirname) || dirname.ends_with(".egg-info")
}

fn should_skip_file(path: &Path) -> bool {
    let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    if SKIP_FILENAMES.contains(&filename) {
        return true;
    }

    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext.to_ascii_lowercase()))
        .unwrap_or_default();
    !READABLE_EXTENSIONS
        .iter()
        .any(|candidate| *candidate == ext)
}

fn normalize_include_paths(include_ignored: &[String]) -> HashSet<String> {
    include_ignored
        .iter()
        .map(|raw| raw.trim().trim_matches('/'))
        .filter(|raw| !raw.is_empty())
        .map(|raw| Path::new(raw).to_string_lossy().replace('\\', "/"))
        .collect()
}

fn is_exact_force_include(
    path: &Path,
    project_path: &Path,
    include_paths: &HashSet<String>,
) -> bool {
    if include_paths.is_empty() {
        return false;
    }

    path.strip_prefix(project_path)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .is_some_and(|relative| include_paths.contains(relative.trim_matches('/')))
}

fn is_force_include(path: &Path, project_path: &Path, include_paths: &HashSet<String>) -> bool {
    if include_paths.is_empty() {
        return false;
    }

    path.strip_prefix(project_path)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .is_some_and(|relative| {
            let relative = relative.trim_matches('/');
            include_paths
                .iter()
                .any(|include| relative == include || relative.starts_with(&format!("{include}/")))
        })
}

struct ConversationDrawerContext<'a> {
    wing: &'a str,
    source_file: &'a str,
    source_path: &'a str,
    source_hash: &'a str,
    source_mtime: Option<f64>,
    agent: &'a str,
    filed_at: &'a str,
    extract_mode: &'a str,
}

fn build_conversation_drawers(
    context: &ConversationDrawerContext<'_>,
    chunks: &[ConversationChunk],
) -> Vec<DrawerInput> {
    chunks
        .iter()
        .map(|chunk| DrawerInput {
            id: format!(
                "drawer_{}_{}_{}",
                sanitize_slug(context.wing),
                sanitize_slug(&chunk.room),
                blake3::hash(format!("{}:{}", context.source_path, chunk.chunk_index).as_bytes())
                    .to_hex(),
            ),
            wing: context.wing.to_string(),
            room: chunk.room.clone(),
            source_file: context.source_file.to_string(),
            source_path: context.source_path.to_string(),
            source_hash: context.source_hash.to_string(),
            source_mtime: context.source_mtime,
            chunk_index: chunk.chunk_index,
            added_by: context.agent.to_string(),
            filed_at: context.filed_at.to_string(),
            ingest_mode: "convos".to_string(),
            extract_mode: context.extract_mode.to_string(),
            text: chunk.content.clone(),
        })
        .collect()
}

fn default_convo_wing(dir: &Path) -> String {
    dir.file_name()
        .map(|name| {
            name.to_string_lossy()
                .to_ascii_lowercase()
                .replace([' ', '-'], "_")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "conversations".to_string())
}

fn sanitize_slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn normalize_search_hits(mut hits: Vec<SearchHit>) -> Vec<SearchHit> {
    for hit in &mut hits {
        hit.source_file = normalize_source_file(&hit.source_file, &hit.source_path);
        hit.similarity = hit.similarity.map(round_similarity);
    }

    hits.sort_by(compare_search_hits);
    hits
}

fn default_identity_text() -> String {
    "## L0 — IDENTITY\nNo identity configured. Create <palace>/identity.txt".to_string()
}

fn render_layer1(drawers: &[DrawerRecord], wing: Option<&str>) -> String {
    if drawers.is_empty() {
        return if let Some(wing_name) = wing {
            format!("## L1 — No memories yet for wing '{wing_name}'.")
        } else {
            "## L1 — No memories yet.".to_string()
        };
    }

    let mut by_room: BTreeMap<String, Vec<&DrawerRecord>> = BTreeMap::new();
    for drawer in drawers {
        by_room.entry(drawer.room.clone()).or_default().push(drawer);
    }

    let mut lines = vec!["## L1 — ESSENTIAL STORY".to_string()];
    for (room, entries) in by_room {
        lines.push(format!("\n[{room}]"));
        for drawer in entries.into_iter().take(4) {
            let mut snippet = drawer.text.replace('\n', " ").trim().to_string();
            if snippet.chars().count() > 200 {
                snippet = format!("{}...", snippet.chars().take(197).collect::<String>());
            }
            let mut line = format!("  - {snippet}");
            if !drawer.source_file.is_empty() {
                line.push_str(&format!("  ({})", drawer.source_file));
            }
            lines.push(line);
        }
    }

    lines.join("\n")
}

fn render_layer2(drawers: &[SearchHit], wing: Option<&str>, room: Option<&str>) -> String {
    if drawers.is_empty() {
        let mut label = String::new();
        if let Some(wing_name) = wing {
            label.push_str(&format!("wing={wing_name}"));
        }
        if let Some(room_name) = room {
            if !label.is_empty() {
                label.push(' ');
            }
            label.push_str(&format!("room={room_name}"));
        }
        if label.is_empty() {
            "No drawers found.".to_string()
        } else {
            format!("No drawers found for {label}.")
        }
    } else {
        let mut lines = vec![format!("## L2 — ON-DEMAND ({} drawers)", drawers.len())];
        for hit in drawers {
            let mut snippet = hit.text.trim().replace('\n', " ");
            if snippet.len() > 300 {
                snippet.truncate(297);
                snippet.push_str("...");
            }
            let mut entry = format!("  [{}] {}", hit.room, snippet);
            if !hit.source_file.is_empty() {
                entry.push_str(&format!("  ({})", hit.source_file));
            }
            lines.push(entry);
        }
        lines.join("\n")
    }
}

#[derive(Clone, Debug, Default)]
struct GraphNodeData {
    wings: BTreeSet<String>,
    halls: BTreeSet<String>,
    count: usize,
    recent: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct RoomGraph {
    nodes: BTreeMap<String, GraphNodeData>,
    total_edges: usize,
}

fn build_room_graph(rows: &[GraphRoomRow]) -> RoomGraph {
    let mut nodes: BTreeMap<String, GraphNodeData> = BTreeMap::new();
    for row in rows {
        let node = nodes.entry(row.room.clone()).or_default();
        node.wings.insert(row.wing.clone());
        node.count += 1;
        if let Some(filed_at) = &row.filed_at
            && node
                .recent
                .as_ref()
                .is_none_or(|current| filed_at > current)
        {
            node.recent = Some(filed_at.clone());
        }
    }

    let total_edges = nodes
        .values()
        .map(|data| {
            let wing_count = data.wings.len();
            if wing_count >= 2 {
                wing_count * (wing_count - 1) / 2
            } else {
                0
            }
        })
        .sum();

    RoomGraph { nodes, total_edges }
}

fn fuzzy_match_room(query: &str, nodes: &BTreeMap<String, GraphNodeData>) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let query_words = query_lower.split('-').collect::<Vec<_>>();
    let mut matches = nodes
        .keys()
        .filter_map(|room| {
            let room_lower = room.to_lowercase();
            if room_lower.contains(&query_lower) {
                return Some((room.clone(), 1u8));
            }
            if query_words
                .iter()
                .any(|word| !word.is_empty() && room_lower.contains(word))
            {
                return Some((room.clone(), 2u8));
            }
            None
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(&right.0)));
    matches.into_iter().map(|(room, _)| room).take(5).collect()
}

fn normalize_source_file(source_file: &str, source_path: &str) -> String {
    let candidate = if source_file.is_empty() {
        source_path
    } else {
        source_file
    };

    Path::new(candidate)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| {
            if candidate.is_empty() {
                "?".to_string()
            } else {
                candidate.to_string()
            }
        })
}

fn round_similarity(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn compare_search_hits(left: &SearchHit, right: &SearchHit) -> std::cmp::Ordering {
    right
        .similarity
        .unwrap_or(f64::NEG_INFINITY)
        .total_cmp(&left.similarity.unwrap_or(f64::NEG_INFINITY))
        .then_with(|| left.source_file.cmp(&right.source_file))
        .then_with(|| left.chunk_index.cmp(&right.chunk_index))
        .then_with(|| left.id.cmp(&right.id))
}

fn sanitize_name(value: &str, field_name: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} must be a non-empty string"
        )));
    }

    if trimmed.len() > 128 {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} exceeds maximum length of 128 characters"
        )));
    }

    if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} contains invalid path characters"
        )));
    }

    if trimmed.contains('\0') {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} contains null bytes"
        )));
    }

    let valid = trimmed.chars().enumerate().all(|(idx, ch)| {
        let allowed = ch.is_ascii_alphanumeric() || matches!(ch, '_' | ' ' | '.' | '\'' | '-');
        if !allowed {
            return false;
        }
        if (idx == 0 || idx == trimmed.len() - 1) && !ch.is_ascii_alphanumeric() {
            return false;
        }
        true
    });

    if !valid {
        return Err(MempalaceError::InvalidArgument(format!(
            "{field_name} contains invalid characters"
        )));
    }

    Ok(trimmed.to_string())
}

fn sanitize_content(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MempalaceError::InvalidArgument(
            "content cannot be empty".to_string(),
        ));
    }
    if trimmed.len() > 100_000 {
        return Err(MempalaceError::InvalidArgument(
            "content exceeds maximum length of 100000 characters".to_string(),
        ));
    }
    if trimmed.contains('\0') {
        return Err(MempalaceError::InvalidArgument(
            "content contains null bytes".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn identifier_fragment(value: &str) -> String {
    let fragment = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let fragment = fragment.trim_matches('-').to_string();
    if fragment.is_empty() {
        "item".to_string()
    } else {
        fragment
    }
}

fn drawer_input_from_record(record: &DrawerRecord) -> DrawerInput {
    DrawerInput {
        id: record.id.clone(),
        wing: record.wing.clone(),
        room: record.room.clone(),
        source_file: record.source_file.clone(),
        source_path: record.source_path.clone(),
        source_hash: record.source_hash.clone(),
        source_mtime: record.source_mtime,
        chunk_index: record.chunk_index,
        added_by: record.added_by.clone(),
        filed_at: record.filed_at.clone(),
        ingest_mode: record.ingest_mode.clone(),
        extract_mode: record.extract_mode.clone(),
        text: record.text.clone(),
    }
}

fn cosine_distance(left: &[f32], right: &[f32]) -> f64 {
    let mut dot = 0.0f64;
    let mut left_norm = 0.0f64;
    let mut right_norm = 0.0f64;
    for (lhs, rhs) in left.iter().zip(right.iter()) {
        let lhs = *lhs as f64;
        let rhs = *rhs as f64;
        dot += lhs * rhs;
        left_norm += lhs * lhs;
        right_norm += rhs * rhs;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return 1.0;
    }
    let similarity = dot / (left_norm.sqrt() * right_norm.sqrt());
    1.0 - similarity.clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

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
