use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct DrawerInput {
    pub id: String,
    pub wing: String,
    pub room: String,
    pub source_file: String,
    pub source_path: String,
    pub source_hash: String,
    pub source_mtime: Option<f64>,
    pub chunk_index: i32,
    pub added_by: String,
    pub filed_at: String,
    pub ingest_mode: String,
    pub extract_mode: String,
    pub importance: Option<f64>,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub id: String,
    pub text: String,
    pub wing: String,
    pub room: String,
    pub source_file: String,
    pub source_path: String,
    pub source_mtime: Option<f64>,
    pub chunk_index: i32,
    pub added_by: Option<String>,
    pub filed_at: Option<String>,
    pub similarity: Option<f64>,
    pub score: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CompressedDrawer {
    pub drawer_id: String,
    pub wing: String,
    pub room: String,
    pub source_file: String,
    pub source_path: String,
    pub ingest_mode: String,
    pub extract_mode: String,
    pub aaak: String,
    pub original_tokens: usize,
    pub compressed_tokens: usize,
    pub compression_ratio: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchFilters {
    pub wing: Option<String>,
    pub room: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchResults {
    pub query: String,
    pub filters: SearchFilters,
    pub results: Vec<SearchHit>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub kind: String,
    pub total_drawers: usize,
    pub wings: BTreeMap<String, usize>,
    pub rooms: BTreeMap<String, usize>,
    pub palace_path: String,
    pub sqlite_path: String,
    pub lance_path: String,
    pub version: String,
    pub schema_version: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rooms {
    pub wing: String,
    pub rooms: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Taxonomy {
    pub taxonomy: BTreeMap<String, BTreeMap<String, usize>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphTraversalNode {
    pub room: String,
    pub wings: Vec<String>,
    pub halls: Vec<String>,
    pub count: usize,
    pub hop: usize,
    pub connected_via: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphTraversalError {
    pub error: String,
    pub suggestions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum GraphTraversalResult {
    Results(Vec<GraphTraversalNode>),
    Error(GraphTraversalError),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TunnelRoom {
    pub room: String,
    pub wings: Vec<String>,
    pub halls: Vec<String>,
    pub count: usize,
    pub recent: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphStatsTunnel {
    pub room: String,
    pub wings: Vec<String>,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GraphStats {
    pub total_rooms: usize,
    pub tunnel_rooms: usize,
    pub total_edges: usize,
    pub rooms_per_wing: BTreeMap<String, usize>,
    pub top_tunnels: Vec<GraphStatsTunnel>,
}
