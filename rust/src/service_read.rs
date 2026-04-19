use std::collections::BTreeMap;

use crate::error::Result;
use crate::model::{
    GraphStats, GraphTraversalResult, LayerStatusSummary, RecallSummary, Rooms, SearchResults,
    Status, Taxonomy, TunnelRoom, WakeUpSummary,
};
use crate::palace_read::PalaceReadRuntime;
use crate::service::App;

impl App {
    pub async fn status(&self) -> Result<Status> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .status()
        .await
    }

    pub async fn list_wings(&self) -> Result<BTreeMap<String, usize>> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .list_wings()
        .await
    }

    pub async fn list_rooms(&self, wing: Option<&str>) -> Result<Rooms> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .list_rooms(wing)
        .await
    }

    pub async fn taxonomy(&self) -> Result<Taxonomy> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .taxonomy()
        .await
    }

    pub async fn traverse_graph(
        &self,
        start_room: &str,
        max_hops: usize,
    ) -> Result<GraphTraversalResult> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .traverse_graph(start_room, max_hops)
        .await
    }

    pub async fn find_tunnels(
        &self,
        wing_a: Option<&str>,
        wing_b: Option<&str>,
    ) -> Result<Vec<TunnelRoom>> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .find_tunnels(wing_a, wing_b)
        .await
    }

    pub async fn graph_stats(&self) -> Result<GraphStats> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .graph_stats()
        .await
    }

    pub async fn search(
        &self,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Result<SearchResults> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .search(query, wing, room, limit)
        .await
    }

    pub async fn wake_up(&self, wing: Option<&str>) -> Result<WakeUpSummary> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .wake_up(wing)
        .await
    }

    pub async fn recall(
        &self,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> Result<RecallSummary> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .recall(wing, room, n_results)
        .await
    }

    pub async fn layer_status(&self) -> Result<LayerStatusSummary> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .layer_status()
        .await
    }
}
