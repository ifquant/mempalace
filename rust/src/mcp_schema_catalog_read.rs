use serde_json::{Value, json};

use crate::mcp_schema::tool;

pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "mempalace_status",
            "Palace overview — total drawers, wing and room counts",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "mempalace_list_wings",
            "List all wings with drawer counts",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "mempalace_list_rooms",
            "List rooms within a wing (or all rooms if no wing given)",
            json!({
                "type": "object",
                "properties": {
                    "wing": {"type":"string","description":"Wing to list rooms for (optional)"}
                }
            }),
        ),
        tool(
            "mempalace_get_taxonomy",
            "Full taxonomy: wing → room → drawer count",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "mempalace_search",
            "Semantic search. Returns verbatim drawer content with similarity scores.",
            json!({
                "type": "object",
                "properties": {
                    "query": {"type":"string","description":"What to search for"},
                    "limit": {"type":"integer","description":"Max results (default 5)"},
                    "wing": {"type":"string","description":"Filter by wing (optional)"},
                    "room": {"type":"string","description":"Filter by room (optional)"}
                },
                "required": ["query"]
            }),
        ),
        tool(
            "mempalace_check_duplicate",
            "Check whether content is already present in the palace using similarity search.",
            json!({
                "type": "object",
                "properties": {
                    "content": {"type":"string","description":"Content to compare against existing drawers"},
                    "threshold": {"type":"number","description":"Minimum similarity threshold (default 0.9)"}
                },
                "required": ["content"]
            }),
        ),
        tool(
            "mempalace_get_aaak_spec",
            "Return the AAAK dialect specification.",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "mempalace_wake_up",
            "Return Layer 0 + Layer 1 context for fast memory wake-up.",
            json!({
                "type": "object",
                "properties": {
                    "wing": {"type":"string","description":"Optional wing filter for the recent essential story"}
                }
            }),
        ),
        tool(
            "mempalace_recall",
            "Return Layer 2 recall results for one wing/room without semantic search.",
            json!({
                "type": "object",
                "properties": {
                    "wing": {"type":"string","description":"Optional wing filter"},
                    "room": {"type":"string","description":"Optional room filter"},
                    "limit": {"type":"integer","description":"Max drawers to return (default 10)"}
                }
            }),
        ),
        tool(
            "mempalace_layers_status",
            "Return Layer 0-3 stack status for the current palace.",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "mempalace_kg_query",
            "Query the knowledge graph for an entity's relationships with optional time and direction filters.",
            json!({
                "type": "object",
                "properties": {
                    "entity": {"type":"string","description":"Entity to query"},
                    "as_of": {"type":"string","description":"Only facts valid at this date (YYYY-MM-DD, optional)"},
                    "direction": {"type":"string","description":"outgoing, incoming, or both (default: both)"}
                },
                "required": ["entity"]
            }),
        ),
        tool(
            "mempalace_kg_timeline",
            "Chronological timeline of facts for one entity or the whole palace.",
            json!({
                "type": "object",
                "properties": {
                    "entity": {"type":"string","description":"Entity to get timeline for (optional)"}
                }
            }),
        ),
        tool(
            "mempalace_kg_stats",
            "Knowledge graph overview: entities, triples, current vs expired facts, relationship types.",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "mempalace_diary_read",
            "Read recent diary entries for an agent.",
            json!({
                "type": "object",
                "properties": {
                    "agent_name": {"type":"string","description":"Agent name"},
                    "last_n": {"type":"integer","description":"How many recent entries to return (default: 10)"}
                },
                "required": ["agent_name"]
            }),
        ),
        tool(
            "mempalace_traverse",
            "Walk the palace graph from a room. Shows connected ideas across wings — the tunnels.",
            json!({
                "type": "object",
                "properties": {
                    "start_room": {"type":"string","description":"Room to start from (e.g. 'chromadb-setup')"},
                    "max_hops": {"type":"integer","description":"How many connections to follow (default: 2)"}
                },
                "required": ["start_room"]
            }),
        ),
        tool(
            "mempalace_find_tunnels",
            "Find rooms that bridge two wings — the hallways connecting different domains.",
            json!({
                "type": "object",
                "properties": {
                    "wing_a": {"type":"string","description":"First wing (optional)"},
                    "wing_b": {"type":"string","description":"Second wing (optional)"}
                }
            }),
        ),
        tool(
            "mempalace_graph_stats",
            "Palace graph overview: total rooms, tunnel connections, edges between wings.",
            json!({"type":"object","properties":{}}),
        ),
    ]
}
