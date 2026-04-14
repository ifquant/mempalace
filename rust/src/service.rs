use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use ignore::WalkBuilder;
use serde::Deserialize;

use crate::VERSION;
use crate::config::AppConfig;
use crate::embed::{EmbeddingProvider, build_embedder};
use crate::error::{MempalaceError, Result};
use crate::model::{
    DiaryReadResult, DiaryWriteResult, DoctorSummary, DrawerInput, GraphStats, GraphStatsTunnel,
    GraphTraversalError, GraphTraversalNode, GraphTraversalResult, InitSummary, KgQueryResult,
    KgStats, KgTimelineResult, KgTriple, MigrateSummary, MineProgressEvent, MineRequest,
    MineSummary, PrepareEmbeddingSummary, RepairSummary, Rooms, SearchFilters, SearchHit,
    SearchResults, Status, Taxonomy, TunnelRoom,
};
use crate::storage::sqlite::{CURRENT_SCHEMA_VERSION, GraphRoomRow, SqliteStore};
use crate::storage::vector::VectorStore;

const READABLE_EXTENSIONS: &[&str] = &[
    ".txt", ".md", ".py", ".js", ".ts", ".jsx", ".tsx", ".json", ".yaml", ".yml", ".html", ".css",
    ".java", ".go", ".rs", ".rb", ".sh", ".csv", ".sql", ".toml",
];

const SKIP_FILENAMES: &[&str] = &[
    "mempalace.yaml",
    "mempalace.yml",
    "mempal.yaml",
    "mempal.yml",
    ".gitignore",
    "package-lock.json",
];

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "env",
    "dist",
    "build",
    ".next",
    "coverage",
    ".mempalace",
    ".ruff_cache",
    ".mypy_cache",
    ".pytest_cache",
    ".cache",
    ".tox",
    ".nox",
    ".idea",
    ".vscode",
    ".ipynb_checkpoints",
    ".eggs",
    "htmlcov",
    "target",
];

const CHUNK_SIZE: usize = 800;
const CHUNK_OVERLAP: usize = 100;
const MIN_CHUNK_SIZE: usize = 50;
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
struct ProjectConfig {
    wing: Option<String>,
    rooms: Option<Vec<ProjectRoom>>,
}

#[derive(Clone, Debug, Deserialize)]
struct ProjectRoom {
    name: String,
    #[serde(default)]
    keywords: Vec<String>,
}

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
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        let _ = vector
            .ensure_table(self.embedder.profile().dimension)
            .await?;
        let schema_version = sqlite.schema_version()?.unwrap_or(CURRENT_SCHEMA_VERSION);

        Ok(InitSummary {
            kind: "init".to_string(),
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
            let existing = sqlite.ingested_file_state(&source_path)?;
            let mtime_pair = existing
                .as_ref()
                .and_then(|state| state.source_mtime)
                .zip(source_mtime);
            if mtime_pair.is_some_and(|(stored, current)| stored == current) {
                files_skipped_unchanged += 1;
                continue;
            }

            let source_hash = blake3::hash(contents.as_bytes()).to_hex().to_string();
            if mtime_pair.is_none()
                && existing.as_ref().map(|state| state.content_hash.as_str())
                    == Some(source_hash.as_str())
            {
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
            files_mined,
            drawers_added,
            files_skipped_unchanged,
            room_counts,
            next_hint: "mempalace search \"what you're looking for\"".to_string(),
        })
    }

    pub async fn add_kg_triple(&self, triple: &KgTriple) -> Result<()> {
        self.init().await?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.add_kg_triple(triple)
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

fn detect_room(root: &Path, path: &Path, content: &str, rooms: &[ProjectRoom]) -> String {
    if rooms.is_empty() {
        return "general".to_string();
    }

    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('\\', "/");
    let filename = path
        .file_stem()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let content_lower = content
        .chars()
        .take(2_000)
        .collect::<String>()
        .to_ascii_lowercase();

    for part in relative
        .split('/')
        .filter(|part| !part.is_empty())
        .take_while(|part| !part.contains('.'))
    {
        for room in rooms {
            let mut candidates = vec![room.name.to_ascii_lowercase()];
            candidates.extend(
                room.keywords
                    .iter()
                    .map(|keyword| keyword.to_ascii_lowercase()),
            );
            if candidates.iter().any(|candidate| {
                part == candidate || candidate.contains(part) || part.contains(candidate)
            }) {
                return room.name.clone();
            }
        }
    }

    for room in rooms {
        let room_name = room.name.to_ascii_lowercase();
        if filename.contains(&room_name) || room_name.contains(&filename) {
            return room.name.clone();
        }
    }

    let mut best_room = None;
    let mut best_score = 0;
    for room in rooms {
        let mut score = content_lower
            .matches(&room.name.to_ascii_lowercase())
            .count();
        for keyword in &room.keywords {
            score += content_lower.matches(&keyword.to_ascii_lowercase()).count();
        }
        if score > best_score {
            best_score = score;
            best_room = Some(room.name.clone());
        }
    }

    best_room.unwrap_or_else(|| "general".to_string())
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

fn load_project_rooms(project_dir: &Path) -> Result<Vec<ProjectRoom>> {
    let config = load_project_config(project_dir)?;
    Ok(config
        .and_then(|config| config.rooms)
        .filter(|rooms| !rooms.is_empty())
        .unwrap_or_else(|| {
            vec![ProjectRoom {
                name: "general".to_string(),
                keywords: Vec::new(),
            }]
        }))
}

fn load_project_config(project_dir: &Path) -> Result<Option<ProjectConfig>> {
    for name in ["mempalace.yaml", "mempal.yaml"] {
        let path = project_dir.join(name);
        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(path)?;
        let config = serde_yml::from_str::<ProjectConfig>(&content).map_err(|err| {
            MempalaceError::InvalidArgument(format!("Invalid project config: {err}"))
        })?;
        return Ok(Some(config));
    }

    Ok(None)
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

    hits.sort_by(|left, right| compare_search_hits(left, right));
    hits
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
        if let Some(filed_at) = &row.filed_at {
            if node
                .recent
                .as_ref()
                .is_none_or(|current| filed_at > current)
            {
                node.recent = Some(filed_at.clone());
            }
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
            ProjectRoom {
                name: "auth".to_string(),
                keywords: vec!["jwt".to_string(), "token".to_string()],
            },
            ProjectRoom {
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
