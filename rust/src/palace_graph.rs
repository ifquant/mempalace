use std::collections::{BTreeMap, BTreeSet};

use crate::model::{
    GraphStats, GraphStatsTunnel, GraphTraversalError, GraphTraversalNode, GraphTraversalResult,
    TunnelRoom,
};
use crate::storage::sqlite::GraphRoomRow;

#[derive(Clone, Debug, Default)]
pub struct RoomGraph {
    pub nodes: BTreeMap<String, GraphNodeData>,
    pub total_edges: usize,
}

#[derive(Clone, Debug, Default)]
pub struct GraphNodeData {
    pub wings: BTreeSet<String>,
    pub halls: BTreeSet<String>,
    pub count: usize,
    pub recent: Option<String>,
}

pub fn build_room_graph(rows: &[GraphRoomRow]) -> RoomGraph {
    let mut nodes: BTreeMap<String, GraphNodeData> = BTreeMap::new();
    for row in rows {
        let node = nodes.entry(row.room.clone()).or_default();
        node.wings.insert(row.wing.clone());
        node.count += 1;
        if let Some(filed_at) = &row.filed_at
            && node
                .recent
                .as_ref()
                .is_none_or(|current| filed_at > current)
        {
            node.recent = Some(filed_at.clone());
        }
    }

    let total_edges = nodes
        .values()
        .map(|data| {
            let wing_count = data.wings.len();
            if wing_count >= 2 {
                wing_count * (wing_count - 1) / 2
            } else {
                0
            }
        })
        .sum();

    RoomGraph { nodes, total_edges }
}

pub fn traverse_graph(
    graph: &RoomGraph,
    start_room: &str,
    max_hops: usize,
) -> GraphTraversalResult {
    let Some(start) = graph.nodes.get(start_room) else {
        return GraphTraversalResult::Error(GraphTraversalError {
            error: format!("Room '{start_room}' not found"),
            suggestions: fuzzy_match_room(start_room, &graph.nodes),
        });
    };

    let mut visited = BTreeSet::new();
    visited.insert(start_room.to_string());
    let mut results = vec![GraphTraversalNode {
        room: start_room.to_string(),
        wings: start.wings.iter().cloned().collect(),
        halls: start.halls.iter().cloned().collect(),
        count: start.count,
        hop: 0,
        connected_via: None,
    }];

    let mut frontier = vec![(start_room.to_string(), 0usize)];
    while let Some((current_room, depth)) = frontier.first().cloned() {
        frontier.remove(0);
        if depth >= max_hops {
            continue;
        }
        let current = match graph.nodes.get(&current_room) {
            Some(current) => current,
            None => continue,
        };
        for (room, data) in &graph.nodes {
            if visited.contains(room) {
                continue;
            }
            let shared_wings = current
                .wings
                .intersection(&data.wings)
                .cloned()
                .collect::<Vec<_>>();
            if shared_wings.is_empty() {
                continue;
            }
            visited.insert(room.clone());
            results.push(GraphTraversalNode {
                room: room.clone(),
                wings: data.wings.iter().cloned().collect(),
                halls: data.halls.iter().cloned().collect(),
                count: data.count,
                hop: depth + 1,
                connected_via: Some(shared_wings),
            });
            if depth + 1 < max_hops {
                frontier.push((room.clone(), depth + 1));
            }
        }
    }

    results.sort_by(|left, right| {
        left.hop
            .cmp(&right.hop)
            .then(right.count.cmp(&left.count))
            .then(left.room.cmp(&right.room))
    });
    results.truncate(50);
    GraphTraversalResult::Results(results)
}

pub fn find_tunnels(
    graph: &RoomGraph,
    wing_a: Option<&str>,
    wing_b: Option<&str>,
) -> Vec<TunnelRoom> {
    let mut tunnels = graph
        .nodes
        .iter()
        .filter_map(|(room, data)| {
            if data.wings.len() < 2 {
                return None;
            }
            if let Some(wing) = wing_a
                && !data.wings.contains(wing)
            {
                return None;
            }
            if let Some(wing) = wing_b
                && !data.wings.contains(wing)
            {
                return None;
            }
            Some(TunnelRoom {
                room: room.clone(),
                wings: data.wings.iter().cloned().collect(),
                halls: data.halls.iter().cloned().collect(),
                count: data.count,
                recent: data.recent.clone().unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();

    tunnels.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then(left.room.cmp(&right.room))
    });
    tunnels.truncate(50);
    tunnels
}

pub fn graph_stats(graph: &RoomGraph) -> GraphStats {
    let tunnel_rooms = graph
        .nodes
        .values()
        .filter(|node| node.wings.len() >= 2)
        .count();

    let mut rooms_per_wing = BTreeMap::new();
    for node in graph.nodes.values() {
        for wing in &node.wings {
            *rooms_per_wing.entry(wing.clone()).or_insert(0) += 1;
        }
    }

    let mut top_tunnels = graph
        .nodes
        .iter()
        .filter(|(_, data)| data.wings.len() >= 2)
        .map(|(room, data)| GraphStatsTunnel {
            room: room.clone(),
            wings: data.wings.iter().cloned().collect(),
            count: data.count,
        })
        .collect::<Vec<_>>();
    top_tunnels.sort_by(|left, right| {
        right
            .wings
            .len()
            .cmp(&left.wings.len())
            .then(right.count.cmp(&left.count))
            .then(left.room.cmp(&right.room))
    });
    top_tunnels.truncate(10);

    GraphStats {
        total_rooms: graph.nodes.len(),
        tunnel_rooms,
        total_edges: graph.total_edges,
        rooms_per_wing,
        top_tunnels,
    }
}

pub fn fuzzy_match_room(query: &str, nodes: &BTreeMap<String, GraphNodeData>) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let query_words = query_lower.split('-').collect::<Vec<_>>();
    let mut matches = nodes
        .keys()
        .filter_map(|room| {
            let room_lower = room.to_lowercase();
            if room_lower.contains(&query_lower) {
                return Some((room.clone(), 1u8));
            }
            if query_words
                .iter()
                .any(|word| !word.is_empty() && room_lower.contains(word))
            {
                return Some((room.clone(), 2u8));
            }
            None
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(&right.0)));
    matches.into_iter().map(|(room, _)| room).take(5).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rows() -> Vec<GraphRoomRow> {
        vec![
            GraphRoomRow {
                room: "deploy".to_string(),
                wing: "project".to_string(),
                filed_at: Some("2026-04-18T08:00:00Z".to_string()),
            },
            GraphRoomRow {
                room: "deploy".to_string(),
                wing: "ops".to_string(),
                filed_at: Some("2026-04-18T09:00:00Z".to_string()),
            },
            GraphRoomRow {
                room: "auth".to_string(),
                wing: "project".to_string(),
                filed_at: Some("2026-04-18T07:00:00Z".to_string()),
            },
        ]
    }

    #[test]
    fn traverse_graph_returns_python_style_results() {
        let graph = build_room_graph(&sample_rows());
        let result = traverse_graph(&graph, "deploy", 2);
        let GraphTraversalResult::Results(results) = result else {
            panic!("expected traversal results");
        };
        assert_eq!(results[0].room, "deploy");
        assert!(results.iter().any(|node| node.room == "auth"));
    }

    #[test]
    fn find_tunnels_filters_cross_wing_rooms() {
        let graph = build_room_graph(&sample_rows());
        let tunnels = find_tunnels(&graph, Some("project"), Some("ops"));
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0].room, "deploy");
    }
}
