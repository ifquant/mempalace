use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::config::AppConfig;
use crate::error::Result;
use crate::model::{LayerStatusSummary, RecallSummary, SearchHit, SearchResults, WakeUpSummary};
use crate::service::App;
use crate::storage::sqlite::DrawerRecord;

#[derive(Clone, Debug, PartialEq)]
pub struct Layer0State {
    pub identity_path: String,
    pub identity: String,
    pub token_estimate: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer1State {
    pub wing: Option<String>,
    pub text: String,
    pub token_estimate: usize,
}

#[derive(Clone)]
pub struct LayerStack {
    app: App,
}

impl LayerStack {
    pub fn new(config: AppConfig) -> Result<Self> {
        Ok(Self {
            app: App::new(config)?,
        })
    }

    pub fn with_app(app: App) -> Self {
        Self { app }
    }

    pub async fn layer0(&self) -> Result<Layer0State> {
        let wake = self.app.wake_up(None).await?;
        Ok(Layer0State {
            identity_path: wake.identity_path,
            token_estimate: wake.identity.split_whitespace().count(),
            identity: wake.identity,
        })
    }

    pub async fn layer1(&self, wing: Option<&str>) -> Result<Layer1State> {
        let wake = self.app.wake_up(wing).await?;
        Ok(Layer1State {
            wing: wake.wing,
            token_estimate: wake.token_estimate,
            text: wake.layer1,
        })
    }

    pub async fn wake_up(&self, wing: Option<&str>) -> Result<WakeUpSummary> {
        self.app.wake_up(wing).await
    }

    pub async fn recall(
        &self,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> Result<RecallSummary> {
        self.app.recall(wing, room, n_results).await
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

    pub async fn status(&self) -> Result<LayerStatusSummary> {
        self.app.layer_status().await
    }
}

pub fn default_identity_text() -> String {
    "## L0 — IDENTITY\nNo identity configured. Create <palace>/identity.txt".to_string()
}

pub fn read_identity_text(identity_path: &Path) -> String {
    if identity_path.exists() {
        fs::read_to_string(identity_path)
            .map(|text| text.trim().to_string())
            .unwrap_or_else(|_| default_identity_text())
    } else {
        default_identity_text()
    }
}

pub fn render_layer1(drawers: &[DrawerRecord], wing: Option<&str>) -> String {
    if drawers.is_empty() {
        return if let Some(wing_name) = wing {
            format!("## L1 — No memories yet for wing '{wing_name}'.")
        } else {
            "## L1 — No memories yet.".to_string()
        };
    }

    let mut by_room: BTreeMap<String, Vec<&DrawerRecord>> = BTreeMap::new();
    for drawer in drawers {
        by_room.entry(drawer.room.clone()).or_default().push(drawer);
    }

    let mut lines = vec!["## L1 — ESSENTIAL STORY".to_string()];
    for (room, entries) in by_room {
        lines.push(format!("\n[{room}]"));
        for drawer in entries.into_iter().take(4) {
            let mut snippet = drawer.text.replace('\n', " ").trim().to_string();
            if snippet.chars().count() > 200 {
                snippet = format!("{}...", snippet.chars().take(197).collect::<String>());
            }
            let mut line = format!("  - {snippet}");
            if !drawer.source_file.is_empty() {
                line.push_str(&format!("  ({})", drawer.source_file));
            }
            lines.push(line);
        }
    }

    lines.join("\n")
}

pub fn render_layer2(drawers: &[SearchHit], wing: Option<&str>, room: Option<&str>) -> String {
    if drawers.is_empty() {
        let mut label = String::new();
        if let Some(wing_name) = wing {
            label.push_str(&format!("wing={wing_name}"));
        }
        if let Some(room_name) = room {
            if !label.is_empty() {
                label.push(' ');
            }
            label.push_str(&format!("room={room_name}"));
        }
        if label.is_empty() {
            "No drawers found.".to_string()
        } else {
            format!("No drawers found for {label}.")
        }
    } else {
        let mut lines = vec![format!("## L2 — ON-DEMAND ({} drawers)", drawers.len())];
        for hit in drawers {
            let mut snippet = hit.text.trim().replace('\n', " ");
            if snippet.len() > 300 {
                snippet.truncate(297);
                snippet.push_str("...");
            }
            let mut entry = format!("  [{}] {}", hit.room, snippet);
            if !hit.source_file.is_empty() {
                entry.push_str(&format!("  ({})", hit.source_file));
            }
            lines.push(entry);
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::LayerStack;
    use crate::config::{AppConfig, EmbeddingBackend};
    use crate::layers::{default_identity_text, read_identity_text, render_layer1, render_layer2};
    use crate::model::SearchHit;
    use crate::service::App;
    use crate::storage::sqlite::DrawerRecord;

    #[tokio::test]
    async fn layer_stack_exposes_layer0_and_layer1() {
        let tmp = tempdir().unwrap();
        let palace = tmp.path().join("palace");
        let mut config = AppConfig::resolve(Some(palace.clone())).unwrap();
        config.embedding.backend = EmbeddingBackend::Hash;
        std::fs::create_dir_all(&palace).unwrap();
        std::fs::write(
            config.identity_path(),
            "I am Atlas.\nTraits: direct, warm.\nProject: Lantern.",
        )
        .unwrap();

        let app = App::new(config.clone()).unwrap();
        let stack = LayerStack::with_app(app);

        let layer0 = stack.layer0().await.unwrap();
        assert!(layer0.identity.contains("I am Atlas."));
        assert!(layer0.token_estimate > 0);

        let layer1 = stack.layer1(None).await.unwrap();
        assert!(layer1.text.contains("## L1"));
        assert!(layer1.token_estimate > 0);
    }

    #[test]
    fn read_identity_text_uses_default_when_missing() {
        let tmp = tempdir().unwrap();
        let identity = read_identity_text(&tmp.path().join("missing.txt"));
        assert_eq!(identity, default_identity_text());
    }

    #[test]
    fn layer_renderers_match_python_style_text() {
        let drawers = vec![DrawerRecord {
            id: "d1".to_string(),
            wing: "proj".to_string(),
            room: "planning".to_string(),
            source_file: "notes.md".to_string(),
            source_path: "/tmp/notes.md".to_string(),
            source_hash: "h".to_string(),
            source_mtime: None,
            chunk_index: 0,
            added_by: "codex".to_string(),
            filed_at: "2026-04-18T00:00:00Z".to_string(),
            ingest_mode: "projects".to_string(),
            extract_mode: "exchange".to_string(),
            text: "Plan the rollout in phases.".to_string(),
        }];
        let layer1 = render_layer1(&drawers, Some("proj"));
        assert!(layer1.contains("## L1"));
        assert!(layer1.contains("[planning]"));

        let hits = vec![SearchHit {
            id: "d1".to_string(),
            text: "Plan the rollout in phases.".to_string(),
            wing: "proj".to_string(),
            room: "planning".to_string(),
            source_file: "notes.md".to_string(),
            source_path: "/tmp/notes.md".to_string(),
            source_mtime: None,
            chunk_index: 0,
            added_by: Some("codex".to_string()),
            filed_at: Some("2026-04-18T00:00:00Z".to_string()),
            similarity: None,
            score: None,
        }];
        let layer2 = render_layer2(&hits, Some("proj"), Some("planning"));
        assert!(layer2.contains("## L2"));
        assert!(layer2.contains("[planning]"));
    }
}
