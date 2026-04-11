use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ignore::WalkBuilder;

use crate::VERSION;
use crate::config::AppConfig;
use crate::embed::{EmbeddingProvider, build_embedder};
use crate::error::{MempalaceError, Result};
use crate::model::{
    DoctorSummary, DrawerInput, InitSummary, KgTriple, MineSummary, Rooms, SearchResults, Status,
    Taxonomy,
};
use crate::storage::sqlite::SqliteStore;
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
            total_drawers: sqlite.total_drawers()?,
            wings: sqlite.list_wings()?,
            rooms: sqlite.list_rooms(None)?.rooms,
            palace_path: self.config.palace_path.display().to_string(),
            version: VERSION.to_string(),
        })
    }

    pub async fn doctor(&self, warm_embedding: bool) -> Result<DoctorSummary> {
        self.config.ensure_dirs()?;
        Ok(self.embedder.doctor(
            &self.config.palace_path.display().to_string(),
            warm_embedding,
        ))
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
        Ok(SearchResults { results: hits })
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
            dir.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "project".to_string())
        });

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

            let room = derive_room(dir, &path);
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

    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for result in builder.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_file() && seen.insert(path.to_path_buf()) {
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

fn derive_room(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .parent()
        .and_then(|parent| {
            if parent.as_os_str().is_empty() {
                None
            } else {
                parent
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            }
        })
        .unwrap_or_else(|| "root".to_string())
}

fn chunk_text(text: &str) -> Vec<String> {
    const MAX_CHARS: usize = 1200;

    let mut chunks = Vec::new();
    let mut current = String::new();

    for paragraph in text.split("\n\n") {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !current.is_empty() && current.len() + trimmed.len() + 2 > MAX_CHARS {
            chunks.push(current.trim().to_string());
            current.clear();
        }

        if trimmed.len() > MAX_CHARS {
            for window in trimmed.as_bytes().chunks(MAX_CHARS) {
                let part = String::from_utf8_lossy(window).trim().to_string();
                if !part.is_empty() {
                    chunks.push(part);
                }
            }
            continue;
        }

        if !current.is_empty() {
            current.push_str("\n\n");
        }
        current.push_str(trimmed);
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    chunks
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
    fn derive_room_uses_parent_folder() {
        let root = Path::new("/tmp/project");
        let path = Path::new("/tmp/project/src/lib.rs");
        assert_eq!(derive_room(root, path), "src");
    }
}
