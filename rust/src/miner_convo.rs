//! Conversation mining pipeline.
//!
//! This path normalizes transcript-like exports first, then routes the text
//! into either exchange extraction or general-memory extraction before replacing
//! the stored source state.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;

use crate::VERSION;
use crate::config::AppConfig;
use crate::convo::{
    MIN_CONVO_CHUNK_SIZE, detect_convo_room, exchange_rooms, extract_exchange_chunks,
    extract_general_memories, general_rooms, scan_convo_files,
};
use crate::embed::EmbeddingProvider;
use crate::error::{MempalaceError, Result};
use crate::miner_support::{
    ConversationDrawerContext, build_conversation_drawers, default_convo_wing,
};
use crate::model::{MineProgressEvent, MineRequest, MineSummary, SearchFilters};
use crate::normalize::normalize_conversation_file;
use crate::palace::{SKIP_DIRS, source_state_matches};
use crate::storage::sqlite::SqliteStore;
use crate::storage::vector::VectorStore;

/// Mines conversation exports or transcript files into drawers.
///
/// `extract = "exchange"` preserves paired conversational turns, while
/// `extract = "general"` reclassifies higher-level memories such as decisions
/// and milestones. In both cases, normalized content is treated as the source
/// of truth for change detection and replacement.
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
            // General extraction is for memory-like statements, not turn-paired
            // transcript playback.
            extract_general_memories(&normalized, 0.3)
        } else {
            // Exchange extraction keeps the transcript boundary and rooming in
            // the conversation-specific pipeline.
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
            // Re-mining is replace-only for a source so stale chunks from an
            // earlier normalization/extraction pass do not survive.
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
