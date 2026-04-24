use std::collections::{BTreeMap, HashMap};

use crate::model::{DedupSourceResult, DedupSummary};
use crate::storage::sqlite::DrawerRecord;
use crate::storage::vector::VectorDrawer;

const MIN_DOC_CHARS: usize = 20;

pub struct Deduplicator<'a> {
    sqlite_drawers: &'a [DrawerRecord],
    vector_drawers: &'a [VectorDrawer],
}

impl<'a> Deduplicator<'a> {
    pub fn new(sqlite_drawers: &'a [DrawerRecord], vector_drawers: &'a [VectorDrawer]) -> Self {
        Self {
            sqlite_drawers,
            vector_drawers,
        }
    }

    pub fn plan(
        &self,
        threshold: f64,
        source_pattern: Option<&str>,
        min_count: usize,
    ) -> DedupPlan {
        let vectors_by_id = self
            .vector_drawers
            .iter()
            .map(|drawer| (drawer.id.clone(), drawer))
            .collect::<HashMap<_, _>>();

        let mut grouped = BTreeMap::<String, Vec<&DrawerRecord>>::new();
        for drawer in self.sqlite_drawers {
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
                if record.text.chars().count() < MIN_DOC_CHARS {
                    delete_ids.push(record.id.clone());
                    local_deleted += 1;
                    continue;
                }

                let Some(vector_record) = vectors_by_id.get(&record.id) else {
                    local_kept += 1;
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

        DedupPlan {
            total_drawers,
            kept,
            delete_ids,
            groups,
        }
    }
}

pub struct DedupPlan {
    pub total_drawers: usize,
    pub kept: usize,
    pub delete_ids: Vec<String>,
    pub groups: Vec<DedupSourceResult>,
}

pub struct DedupSummaryContext {
    pub kind: String,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub threshold: f64,
    pub dry_run: bool,
    pub wing: Option<String>,
    pub source: Option<String>,
    pub min_count: usize,
    pub stats_only: bool,
}

impl DedupPlan {
    pub fn into_summary(self, context: DedupSummaryContext) -> DedupSummary {
        DedupSummary {
            kind: context.kind,
            palace_path: context.palace_path,
            sqlite_path: context.sqlite_path,
            lance_path: context.lance_path,
            version: context.version,
            threshold: context.threshold,
            dry_run: context.dry_run,
            wing: context.wing,
            source: context.source,
            min_count: context.min_count,
            sources_checked: self.groups.len(),
            total_drawers: self.total_drawers,
            kept: self.kept,
            deleted: self.delete_ids.len(),
            stats_only: context.stats_only,
            groups: self.groups,
        }
    }
}

pub fn cosine_distance(left: &[f32], right: &[f32]) -> f64 {
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

    fn drawer(id: &str, source_file: &str, text: &str) -> DrawerRecord {
        DrawerRecord {
            id: id.to_string(),
            wing: "project".to_string(),
            room: "general".to_string(),
            source_file: source_file.to_string(),
            source_path: format!("/tmp/{source_file}"),
            source_hash: id.to_string(),
            source_mtime: None,
            chunk_index: 0,
            added_by: "codex".to_string(),
            filed_at: "2026-04-18T00:00:00Z".to_string(),
            ingest_mode: "projects".to_string(),
            extract_mode: "exchange".to_string(),
            text: text.to_string(),
        }
    }

    fn vector(id: &str, source_file: &str, vector: Vec<f32>) -> VectorDrawer {
        VectorDrawer {
            id: id.to_string(),
            wing: "project".to_string(),
            room: "general".to_string(),
            source_file: source_file.to_string(),
            source_path: format!("/tmp/{source_file}"),
            text: "body".to_string(),
            vector,
        }
    }

    #[test]
    fn cosine_distance_reports_zero_for_identical_vectors() {
        assert_eq!(cosine_distance(&[1.0, 0.0], &[1.0, 0.0]), 0.0);
    }

    #[test]
    fn deduplicator_keeps_longest_and_marks_duplicates() {
        let sqlite_drawers = vec![
            drawer("a", "dup.txt", "this is a longer chunk"),
            drawer("b", "dup.txt", "short chunk"),
        ];
        let vector_drawers = vec![
            vector("a", "dup.txt", vec![1.0, 0.0]),
            vector("b", "dup.txt", vec![1.0, 0.0]),
        ];

        let plan = Deduplicator::new(&sqlite_drawers, &vector_drawers).plan(0.01, None, 2);
        assert_eq!(plan.kept, 1);
        assert_eq!(plan.delete_ids, vec!["b".to_string()]);
        assert_eq!(plan.groups[0].before, 2);
        assert_eq!(plan.groups[0].deleted, 1);
    }
}
