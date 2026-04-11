use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ignore::WalkBuilder;
use serde::Deserialize;

use crate::VERSION;
use crate::config::AppConfig;
use crate::embed::{EmbeddingProvider, build_embedder};
use crate::error::{MempalaceError, Result};
use crate::model::{
    DoctorSummary, DrawerInput, InitSummary, KgTriple, MigrateSummary, MineSummary,
    PrepareEmbeddingSummary, RepairSummary, Rooms, SearchFilters, SearchResults, Status, Taxonomy,
};
use crate::storage::sqlite::{CURRENT_SCHEMA_VERSION, SqliteStore};
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

        Ok(InitSummary {
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
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
        Ok(self.embedder.doctor(
            &self.config.palace_path.display().to_string(),
            warm_embedding,
        ))
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
                    palace_path: self.config.palace_path.display().to_string(),
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
            palace_path: self.config.palace_path.display().to_string(),
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
        let hits = vector.search(&embedding, wing, room, limit).await?;
        Ok(SearchResults {
            query: query.to_string(),
            filters: SearchFilters {
                wing: wing.map(ToOwned::to_owned),
                room: room.map(ToOwned::to_owned),
            },
            results: hits,
        })
    }

    pub async fn mine_project(
        &self,
        dir: &Path,
        wing_override: Option<&str>,
        limit: usize,
        respect_gitignore: bool,
        include_ignored: &[String],
    ) -> Result<MineSummary> {
        if !dir.exists() {
            return Err(MempalaceError::InvalidArgument(format!(
                "Project directory does not exist: {}",
                dir.display()
            )));
        }

        self.init().await?;
        let wing = wing_override.map(ToOwned::to_owned).unwrap_or_else(|| {
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

        let files = discover_files(dir, respect_gitignore, include_ignored)?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        let mut sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;

        let mut files_seen = 0_usize;
        let mut files_mined = 0_usize;
        let mut files_skipped_unchanged = 0_usize;
        let mut drawers_added = 0_usize;

        for path in files
            .into_iter()
            .take(if limit == 0 { usize::MAX } else { limit })
        {
            files_seen += 1;
            let Some(contents) = read_text_file(&path)? else {
                continue;
            };

            let source_path = path.canonicalize()?.display().to_string();
            let source_hash = blake3::hash(contents.as_bytes()).to_hex().to_string();
            if sqlite.source_hash(&source_path)?.as_deref() == Some(source_hash.as_str()) {
                files_skipped_unchanged += 1;
                continue;
            }

            let room = detect_room(dir, &path, &contents, &rooms);
            let chunks = chunk_text(&contents);
            if chunks.is_empty() {
                continue;
            }

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
                    source_path: source_path.clone(),
                    source_hash: source_hash.clone(),
                    chunk_index: idx as i32,
                    text: chunk.clone(),
                })
                .collect();

            let embeddings = self.embedder.embed_documents(&chunks)?;
            vector.replace_source(&drawers, &embeddings).await?;
            sqlite.replace_source(&source_path, &wing, &room, &source_hash, &drawers)?;

            drawers_added += drawers.len();
            files_mined += 1;
        }

        Ok(MineSummary {
            wing,
            files_seen,
            files_mined,
            drawers_added,
            files_skipped_unchanged,
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
}

fn discover_files(
    dir: &Path,
    respect_gitignore: bool,
    include_ignored: &[String],
) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(dir);
    builder.hidden(false);
    builder.git_ignore(respect_gitignore);
    builder.git_global(respect_gitignore);
    builder.git_exclude(respect_gitignore);
    builder.require_git(false);
    builder.filter_entry(|entry| {
        entry
            .file_name()
            .to_str()
            .map(|name| !should_skip_dir(name))
            .unwrap_or(true)
    });

    let include_paths = normalize_include_paths(include_ignored);
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
}
