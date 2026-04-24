use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;

use crate::VERSION;
use crate::config::AppConfig;
use crate::embed::EmbeddingProvider;
use crate::error::{MempalaceError, Result};
use crate::miner::mine_conversations_run;
use crate::miner_support::{chunk_text, discover_files, read_text_file, sanitize_slug};
use crate::model::{DrawerInput, MineProgressEvent, MineRequest, MineSummary, SearchFilters};
use crate::palace::source_state_matches;
use crate::room_detector::{detect_room, load_project_config, load_project_rooms};
use crate::storage::sqlite::SqliteStore;
use crate::storage::vector::VectorStore;

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
                importance: None,
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
