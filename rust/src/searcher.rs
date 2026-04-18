use std::cmp::Ordering;
use std::path::Path;

use crate::config::AppConfig;
use crate::error::Result;
use crate::model::{SearchHit, SearchResults};
use crate::service::App;

#[derive(Clone)]
pub struct Searcher {
    app: App,
}

impl Searcher {
    pub fn new(config: AppConfig) -> Result<Self> {
        Ok(Self {
            app: App::new(config)?,
        })
    }

    pub fn with_app(app: App) -> Self {
        Self { app }
    }

    pub async fn search(
        &self,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> Result<SearchResults> {
        self.app.search(query, wing, room, n_results).await
    }
}

pub fn render_search_human(summary: &SearchResults) -> String {
    if summary.results.is_empty() {
        return format!("\n  No results found for: \"{}\"\n", summary.query);
    }

    let mut out = String::new();
    out.push_str(&format!("\n{}\n", "=".repeat(60)));
    out.push_str(&format!("  Results for: \"{}\"\n", summary.query));
    if let Some(wing) = &summary.filters.wing {
        out.push_str(&format!("  Wing: {wing}\n"));
    }
    if let Some(room) = &summary.filters.room {
        out.push_str(&format!("  Room: {room}\n"));
    }
    out.push_str(&format!("{}\n\n", "=".repeat(60)));

    for (index, hit) in summary.results.iter().enumerate() {
        let similarity = hit
            .similarity
            .map(|value| value.to_string())
            .unwrap_or_else(|| "?".to_string());
        out.push_str(&format!("  [{}] {} / {}\n", index + 1, hit.wing, hit.room));
        out.push_str(&format!("      Source: {}\n", hit.source_file));
        out.push_str(&format!("      Match:  {similarity}\n\n"));
        for line in hit.text.trim().lines() {
            out.push_str(&format!("      {line}\n"));
        }
        out.push('\n');
        out.push_str(&format!("  {}\n", "─".repeat(56)));
    }
    out.push('\n');
    out
}

pub fn normalize_search_hits(mut hits: Vec<SearchHit>) -> Vec<SearchHit> {
    for hit in &mut hits {
        hit.source_file = normalize_source_file(&hit.source_file, &hit.source_path);
        hit.similarity = hit.similarity.map(round_similarity);
    }

    hits.sort_by(compare_search_hits);
    hits
}

pub fn normalize_source_file(source_file: &str, source_path: &str) -> String {
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

pub fn round_similarity(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

pub fn compare_search_hits(left: &SearchHit, right: &SearchHit) -> Ordering {
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
    use tempfile::tempdir;

    use super::{Searcher, normalize_search_hits, render_search_human};
    use crate::config::{AppConfig, EmbeddingBackend};
    use crate::model::{SearchFilters, SearchHit, SearchResults};
    use crate::service::App;

    #[test]
    fn render_search_human_matches_python_style_blocks() {
        let summary = SearchResults {
            query: "GraphQL".to_string(),
            filters: SearchFilters {
                wing: Some("project".to_string()),
                room: Some("backend".to_string()),
            },
            results: vec![SearchHit {
                id: "drawer_1".to_string(),
                text: "Planning notes about GraphQL migration.".to_string(),
                wing: "project".to_string(),
                room: "backend".to_string(),
                source_file: "plan.txt".to_string(),
                source_path: "/tmp/project/plan.txt".to_string(),
                source_mtime: None,
                chunk_index: 0,
                added_by: None,
                filed_at: None,
                similarity: Some(0.982),
                score: Some(0.018),
            }],
        };

        let rendered = render_search_human(&summary);
        assert!(rendered.contains("Results for: \"GraphQL\""));
        assert!(rendered.contains("Wing: project"));
        assert!(rendered.contains("Room: backend"));
        assert!(rendered.contains("Source: plan.txt"));
        assert!(rendered.contains("Match:  0.982"));
        assert!(rendered.contains("Planning notes about GraphQL migration."));
    }

    #[tokio::test]
    async fn searcher_facade_runs_programmatic_search() {
        let tmp = tempdir().unwrap();
        let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
        config.embedding.backend = EmbeddingBackend::Hash;
        let app = App::new(config.clone()).unwrap();
        app.init().await.unwrap();
        app.add_drawer(
            "project",
            "backend",
            "Planning notes about GraphQL migration and deployment rollout.",
            Some("notes/plan.txt"),
            Some("tester"),
        )
        .await
        .unwrap();

        let searcher = Searcher::with_app(app);
        let results = searcher
            .search("GraphQL", Some("project"), Some("backend"), 5)
            .await
            .unwrap();
        assert_eq!(results.query, "GraphQL");
        assert_eq!(results.filters.wing.as_deref(), Some("project"));
        assert_eq!(results.filters.room.as_deref(), Some("backend"));
        assert_eq!(results.results.len(), 1);
        assert_eq!(results.results[0].source_file, "plan.txt");
    }

    #[test]
    fn normalize_search_hits_uses_python_style_similarity_and_basename() {
        let hits = vec![SearchHit {
            id: "drawer_1".to_string(),
            text: "Alpha".to_string(),
            wing: "project".to_string(),
            room: "backend".to_string(),
            source_file: "notes/plan.txt".to_string(),
            source_path: "/tmp/project/notes/plan.txt".to_string(),
            source_mtime: None,
            chunk_index: 0,
            added_by: None,
            filed_at: None,
            similarity: Some(0.9816),
            score: Some(0.0184),
        }];

        let normalized = normalize_search_hits(hits);
        assert_eq!(normalized[0].source_file, "plan.txt");
        assert_eq!(normalized[0].similarity, Some(0.982));
    }

    #[test]
    fn normalize_search_hits_keeps_duplicate_files_as_separate_hits() {
        let hits = vec![
            SearchHit {
                id: "drawer_2".to_string(),
                text: "Beta".to_string(),
                wing: "project".to_string(),
                room: "backend".to_string(),
                source_file: "plan.txt".to_string(),
                source_path: "/tmp/project/plan.txt".to_string(),
                source_mtime: None,
                chunk_index: 1,
                added_by: None,
                filed_at: None,
                similarity: Some(0.7),
                score: Some(0.3),
            },
            SearchHit {
                id: "drawer_1".to_string(),
                text: "Alpha".to_string(),
                wing: "project".to_string(),
                room: "backend".to_string(),
                source_file: "plan.txt".to_string(),
                source_path: "/tmp/project/plan.txt".to_string(),
                source_mtime: None,
                chunk_index: 0,
                added_by: None,
                filed_at: None,
                similarity: Some(0.7),
                score: Some(0.3),
            },
        ];

        let normalized = normalize_search_hits(hits);
        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].id, "drawer_1");
        assert_eq!(normalized[1].id, "drawer_2");
    }
}
