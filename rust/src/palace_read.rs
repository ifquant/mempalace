//! Read-only palace runtime for status, taxonomy, search, and recall flows.
//!
//! This facade opens the durable stores with the configured embedding profile,
//! then delegates pure read surfaces that are safe for CLI, MCP, and tests.

use std::collections::BTreeMap;
use std::fs;

use crate::VERSION;
use crate::config::AppConfig;
use crate::dialect::count_tokens;
use crate::embed::EmbeddingProvider;
use crate::error::Result;
use crate::layers::{read_identity_text, render_layer1, render_layer2};
use crate::model::{
    GraphStats, GraphTraversalResult, LayerStatusSummary, RecallSummary, Rooms, SearchFilters,
    SearchHit, SearchResults, Status, Taxonomy, TunnelRoom, WakeUpSummary,
};
use crate::palace_graph::{
    build_room_graph, find_tunnels as find_graph_tunnels, graph_stats as summarize_graph,
    traverse_graph as traverse_room_graph,
};
use crate::searcher::{normalize_search_hits, normalize_source_file};
use crate::storage::sqlite::{CURRENT_SCHEMA_VERSION, SqliteStore};
use crate::storage::vector::VectorStore;

/// Read-oriented palace facade shared by user-facing status and recall APIs.
pub struct PalaceReadRuntime<'a> {
    pub config: &'a AppConfig,
    pub embedder: &'a dyn EmbeddingProvider,
}

impl<'a> PalaceReadRuntime<'a> {
    fn open_sqlite(&self) -> Result<SqliteStore> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        Ok(sqlite)
    }

    /// Returns a summary of palace paths, schema version, and taxonomy counts.
    pub async fn status(&self) -> Result<Status> {
        let sqlite = self.open_sqlite()?;
        Ok(Status {
            kind: "status".to_string(),
            total_drawers: sqlite.total_drawers()?,
            wings: sqlite.list_wings()?,
            rooms: sqlite.list_rooms(None)?.rooms,
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            lance_path: self.config.lance_path().display().to_string(),
            version: VERSION.to_string(),
            schema_version: sqlite.schema_version()?.unwrap_or(CURRENT_SCHEMA_VERSION),
        })
    }

    /// Lists wings currently present in SQLite drawer metadata.
    pub async fn list_wings(&self) -> Result<BTreeMap<String, usize>> {
        self.open_sqlite()?.list_wings()
    }

    /// Lists rooms for one wing or across the whole palace.
    pub async fn list_rooms(&self, wing: Option<&str>) -> Result<Rooms> {
        self.open_sqlite()?.list_rooms(wing)
    }

    /// Returns the full wing/room taxonomy derived from stored drawers.
    pub async fn taxonomy(&self) -> Result<Taxonomy> {
        self.open_sqlite()?.taxonomy()
    }

    /// Traverses the room graph built from SQLite room adjacency rows.
    pub async fn traverse_graph(
        &self,
        start_room: &str,
        max_hops: usize,
    ) -> Result<GraphTraversalResult> {
        let sqlite = self.open_sqlite()?;
        let graph = build_room_graph(&sqlite.graph_room_rows()?);
        Ok(traverse_room_graph(&graph, start_room, max_hops))
    }

    /// Finds graph tunnel candidates between wings using the current room graph.
    pub async fn find_tunnels(
        &self,
        wing_a: Option<&str>,
        wing_b: Option<&str>,
    ) -> Result<Vec<TunnelRoom>> {
        let sqlite = self.open_sqlite()?;
        let graph = build_room_graph(&sqlite.graph_room_rows()?);
        Ok(find_graph_tunnels(&graph, wing_a, wing_b))
    }

    /// Summarizes graph connectivity and room-level structure.
    pub async fn graph_stats(&self) -> Result<GraphStats> {
        let sqlite = self.open_sqlite()?;
        let graph = build_room_graph(&sqlite.graph_room_rows()?);
        Ok(summarize_graph(&graph))
    }

    /// Runs semantic search against LanceDB, with optional wing/room filters.
    pub async fn search(
        &self,
        query: &str,
        wing: Option<&str>,
        room: Option<&str>,
        limit: usize,
    ) -> Result<SearchResults> {
        let _sqlite = self.open_sqlite()?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        let embedding = self.embedder.embed_query(query)?;
        let hits = normalize_search_hits(vector.search(&embedding, wing, room, limit).await?);
        Ok(SearchResults {
            query: query.to_string(),
            filters: SearchFilters {
                wing: wing.map(ToOwned::to_owned),
                room: room.map(ToOwned::to_owned),
            },
            results: hits,
        })
    }

    /// Builds the wake-up payload from identity text plus recent drawers.
    pub async fn wake_up(&self, wing: Option<&str>) -> Result<WakeUpSummary> {
        let sqlite = self.open_sqlite()?;
        let identity_path = self.config.identity_path();
        let identity = read_identity_text(&identity_path);
        let recent = sqlite.recent_drawers(wing, 15)?;
        let layer1 = render_layer1(&recent, wing);
        let token_estimate = count_tokens(&identity) + count_tokens(&layer1);

        Ok(WakeUpSummary {
            kind: "wake_up".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            wing: wing.map(ToOwned::to_owned),
            identity_path: identity_path.display().to_string(),
            identity,
            layer1,
            token_estimate,
        })
    }

    /// Renders layer-2 recall from ordered SQLite drawer reads.
    pub async fn recall(
        &self,
        wing: Option<&str>,
        room: Option<&str>,
        n_results: usize,
    ) -> Result<RecallSummary> {
        let sqlite = self.open_sqlite()?;
        let matches = sqlite.list_drawers(wing)?;
        let mut hits = matches
            .into_iter()
            .filter(|record| room.map(|value| value == record.room).unwrap_or(true))
            .map(|record| SearchHit {
                id: record.id,
                text: record.text,
                wing: record.wing,
                room: record.room,
                source_file: normalize_source_file(&record.source_file, &record.source_path),
                source_path: record.source_path,
                source_mtime: record.source_mtime,
                chunk_index: record.chunk_index,
                added_by: Some(record.added_by),
                filed_at: Some(record.filed_at),
                similarity: None,
                score: None,
            })
            .collect::<Vec<_>>();

        hits.sort_by(|left, right| {
            left.wing
                .cmp(&right.wing)
                .then_with(|| left.room.cmp(&right.room))
                .then_with(|| left.source_file.cmp(&right.source_file))
                .then_with(|| left.chunk_index.cmp(&right.chunk_index))
        });

        let total_matches = hits.len();
        let n_results = n_results.max(1);
        hits.truncate(n_results);
        let text = render_layer2(&hits, wing, room);

        Ok(RecallSummary {
            kind: "recall".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            wing: wing.map(ToOwned::to_owned),
            room: room.map(ToOwned::to_owned),
            n_results,
            total_matches,
            text,
            results: hits,
        })
    }

    /// Reports whether the layered recall inputs exist and how large they are.
    pub async fn layer_status(&self) -> Result<LayerStatusSummary> {
        let sqlite = self.open_sqlite()?;
        let identity_path = self.config.identity_path();
        let identity_exists = identity_path.exists();
        let identity_text = if identity_exists {
            fs::read_to_string(&identity_path)
                .map(|text| text.trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        Ok(LayerStatusSummary {
            kind: "layers_status".to_string(),
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            identity_path: identity_path.display().to_string(),
            identity_exists,
            identity_tokens: count_tokens(&identity_text),
            total_drawers: sqlite.total_drawers()?,
            layer0_description: "Identity text loaded from palace-local identity.txt".to_string(),
            layer1_description: "Essential story auto-generated from recent drawers".to_string(),
            layer2_description: "On-demand wing/room recall from stored drawers".to_string(),
            layer3_description: "Deep semantic search across the whole palace".to_string(),
        })
    }
}
