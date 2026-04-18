use crate::dialect::{CompressMetadata, CompressionStats, Dialect};
use crate::model::{CompressSummary, CompressedDrawer};
use crate::storage::sqlite::DrawerRecord;

pub struct CompressionRun {
    pub entries: Vec<CompressedDrawer>,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
}

pub struct CompressSummaryContext {
    pub palace_path: String,
    pub sqlite_path: String,
    pub version: String,
    pub wing: Option<String>,
    pub dry_run: bool,
}

impl CompressionRun {
    pub fn from_drawers(drawers: Vec<DrawerRecord>, dialect: &Dialect) -> Self {
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
                compressed_drawer_from_record(drawer, aaak, stats)
            })
            .collect::<Vec<_>>();

        Self {
            entries,
            original_tokens,
            compressed_tokens,
        }
    }

    pub fn into_summary(self, context: CompressSummaryContext) -> CompressSummary {
        CompressSummary {
            kind: "compress".to_string(),
            palace_path: context.palace_path,
            sqlite_path: context.sqlite_path,
            version: context.version,
            wing: context.wing,
            dry_run: context.dry_run,
            processed: self.entries.len(),
            stored: if context.dry_run {
                0
            } else {
                self.entries.len()
            },
            original_tokens: self.original_tokens,
            compressed_tokens: self.compressed_tokens,
            compression_ratio: self.original_tokens as f64 / self.compressed_tokens.max(1) as f64,
            entries: self.entries,
        }
    }
}

fn compressed_drawer_from_record(
    drawer: DrawerRecord,
    aaak: String,
    stats: CompressionStats,
) -> CompressedDrawer {
    CompressedDrawer {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawer(id: &str, room: &str, text: &str) -> DrawerRecord {
        DrawerRecord {
            id: id.to_string(),
            wing: "project".to_string(),
            room: room.to_string(),
            source_file: "notes.txt".to_string(),
            source_path: "/tmp/notes.txt".to_string(),
            source_hash: "hash".to_string(),
            source_mtime: None,
            chunk_index: 0,
            added_by: "codex".to_string(),
            filed_at: "2026-04-18T00:00:00Z".to_string(),
            ingest_mode: "projects".to_string(),
            extract_mode: "exchange".to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn compression_run_builds_entries_and_totals() {
        let run = CompressionRun::from_drawers(
            vec![
                drawer(
                    "a",
                    "backend",
                    "We decided to use GraphQL because schema contracts matter.",
                ),
                drawer(
                    "b",
                    "planning",
                    "Important rollout milestone with phased deployment.",
                ),
            ],
            &Dialect,
        );

        assert_eq!(run.entries.len(), 2);
        assert!(run.original_tokens > 0);
        assert!(run.compressed_tokens > 0);
        assert!(run.entries[0].aaak.contains("project|backend"));
    }
}
