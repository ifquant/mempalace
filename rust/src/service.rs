use std::sync::Arc;

use crate::config::AppConfig;
use crate::embed::{EmbeddingProvider, build_embedder};
use crate::error::Result;

#[derive(Clone)]
pub struct App {
    pub config: AppConfig,
    pub(crate) embedder: Arc<dyn EmbeddingProvider>,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self> {
        let embedder = build_embedder(&config.embedding)?;
        Ok(Self { config, embedder })
    }

    pub fn with_embedder(config: AppConfig, embedder: Arc<dyn EmbeddingProvider>) -> Self {
        Self { config, embedder }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::miner_support::chunk_text;
    use crate::model::SearchHit;
    use crate::room_detector::detect_room;
    use crate::searcher::normalize_search_hits;

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
