//! Read-only `App` helpers for inspecting an existing palace.
//!
//! These methods are the audit entrypoints for search, recall, graph traversal,
//! and status-style flows. Each delegates straight into `PalaceReadRuntime`.

use std::collections::BTreeMap;

use crate::error::Result;
use crate::model::{
    GraphStats, GraphTraversalResult, LayerStatusSummary, RecallSummary, Rooms, SearchResults,
    Status, Taxonomy, TunnelRoom, WakeUpSummary,
};
use crate::palace_read::PalaceReadRuntime;
use crate::service::App;

impl App {
    /// Summarize the current palace contents and health.
    pub async fn status(&self) -> Result<Status> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .status()
        .await
    }

    /// List wing names with drawer counts from the read runtime.
    pub async fn list_wings(&self) -> Result<BTreeMap<String, usize>> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .list_wings()
        .await
    }

    /// List rooms, optionally scoped to one wing.
    pub async fn list_rooms(&self, wing: Option<&str>) -> Result<Rooms> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .list_rooms(wing)
        .await
    }

    /// Return the high-level taxonomy that drives wake-up and browsing flows.
    pub async fn taxonomy(&self) -> Result<Taxonomy> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .taxonomy()
        .await
    }

    /// Traverse the room graph from one starting room.
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

    /// Find candidate tunnel rooms that connect two wings.
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

    /// Return aggregate graph metrics without fetching individual triples.
    pub async fn graph_stats(&self) -> Result<GraphStats> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .graph_stats()
        .await
    }

    /// Run semantic search across drawers, optionally scoped by wing or room.
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

    /// Build the Layer 0/1 wake-up bundle for one wing or the whole palace.
    pub async fn wake_up(&self, wing: Option<&str>) -> Result<WakeUpSummary> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .wake_up(wing)
        .await
    }

    /// Recall recent drawers without a semantic query.
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

    /// Report how much data is present in each wake-up layer.
    pub async fn layer_status(&self) -> Result<LayerStatusSummary> {
        PalaceReadRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .layer_status()
        .await
    }
}
