use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use ignore::WalkBuilder;

use crate::VERSION;
use crate::config::AppConfig;
use crate::convo::{
    ConversationChunk, MIN_CONVO_CHUNK_SIZE, detect_convo_room, exchange_rooms,
    extract_exchange_chunks, extract_general_memories, general_rooms, scan_convo_files,
};
use crate::embed::EmbeddingProvider;
use crate::error::{MempalaceError, Result};
use crate::model::{DrawerInput, MineProgressEvent, MineRequest, MineSummary, SearchFilters};
use crate::normalize::normalize_conversation_file;
use crate::palace::{SKIP_DIRS, source_state_matches};
use crate::room_detector::{detect_room, load_project_config, load_project_rooms};
use crate::storage::sqlite::SqliteStore;
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

pub async fn mine_project_run<F>(
    config: &AppConfig,
    embedder: Arc<dyn EmbeddingProvider>,
    dir: &Path,
    request: &MineRequest,
    mut on_progress: F,
) -> Result<MineSummary>
where
    F: FnMut(MineProgressEvent),
{
    if request.mode == "convos" {
        return mine_conversations_run(config, embedder, dir, request, on_progress).await;
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
        Some(VectorStore::connect(&config.lance_path()).await?)
    };
    let mut sqlite = SqliteStore::open(&config.sqlite_path())?;
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

        let embeddings = embedder.embed_documents(&chunks)?;
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
        palace_path: config.palace_path.display().to_string(),
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

pub async fn mine_conversations_run<F>(
    config: &AppConfig,
    embedder: Arc<dyn EmbeddingProvider>,
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
        Some(VectorStore::connect(&config.lance_path()).await?)
    };
    let mut sqlite = SqliteStore::open(&config.sqlite_path())?;
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
        let embeddings = embedder.embed_documents(&drawer_texts)?;
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
        palace_path: config.palace_path.display().to_string(),
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

pub(crate) fn chunk_text(text: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::chunk_text;

    #[test]
    fn chunk_text_splits_large_input() {
        let input = format!("{}\n\n{}", "a".repeat(900), "b".repeat(900));
        let chunks = chunk_text(&input);
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|chunk| chunk.len() >= 50));
    }
}
