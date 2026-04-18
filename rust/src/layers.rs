use crate::config::AppConfig;
use crate::error::Result;
use crate::model::{LayerStatusSummary, RecallSummary, SearchResults, WakeUpSummary};
use crate::service::App;

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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::LayerStack;
    use crate::config::{AppConfig, EmbeddingBackend};
    use crate::service::App;

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
}
