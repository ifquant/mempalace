use serde_json::{Value, json};

use crate::mcp_schema::tool;

pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "mempalace_add_drawer",
            "File verbatim content into the palace.",
            json!({
                "type": "object",
                "properties": {
                    "wing": {"type":"string","description":"Wing name"},
                    "room": {"type":"string","description":"Room name"},
                    "content": {"type":"string","description":"Verbatim content to store"},
                    "source_file": {"type":"string","description":"Optional original source path or label"},
                    "added_by": {"type":"string","description":"Who is filing this drawer (default: mcp)"}
                },
                "required": ["wing", "room", "content"]
            }),
        ),
        tool(
            "mempalace_delete_drawer",
            "Delete a drawer by ID.",
            json!({
                "type": "object",
                "properties": {
                    "drawer_id": {"type":"string","description":"Drawer ID to delete"}
                },
                "required": ["drawer_id"]
            }),
        ),
        tool(
            "mempalace_repair",
            "Run non-destructive palace repair diagnostics.",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "mempalace_repair_scan",
            "Scan SQLite and LanceDB for drift and write corrupt_ids.txt.",
            json!({
                "type": "object",
                "properties": {
                    "wing": {"type":"string","description":"Optional wing filter for drift scan"}
                }
            }),
        ),
        tool(
            "mempalace_repair_prune",
            "Preview or apply deletion of queued corrupt IDs from corrupt_ids.txt.",
            json!({
                "type": "object",
                "properties": {
                    "confirm": {"type":"boolean","description":"Actually delete queued IDs instead of previewing"}
                }
            }),
        ),
        tool(
            "mempalace_repair_rebuild",
            "Rebuild LanceDB from SQLite drawers using the active embedder profile.",
            json!({"type":"object","properties":{}}),
        ),
        tool(
            "mempalace_compress",
            "Generate AAAK summaries for drawers and optionally store them.",
            json!({
                "type": "object",
                "properties": {
                    "wing": {"type":"string","description":"Optional wing filter"},
                    "dry_run": {"type":"boolean","description":"Preview summaries without storing them"}
                }
            }),
        ),
        tool(
            "mempalace_dedup",
            "Deduplicate near-identical drawers grouped by source_file.",
            json!({
                "type": "object",
                "properties": {
                    "threshold": {"type":"number","description":"Cosine distance threshold (default 0.15)"},
                    "dry_run": {"type":"boolean","description":"Preview without deleting"},
                    "stats_only": {"type":"boolean","description":"Show stats without deleting"},
                    "wing": {"type":"string","description":"Optional wing filter"},
                    "source": {"type":"string","description":"Optional source_file substring filter"},
                    "min_count": {"type":"integer","description":"Minimum group size to consider (default 5)"}
                }
            }),
        ),
        tool(
            "mempalace_kg_add",
            "Add a fact to the knowledge graph. Subject → predicate → object with optional valid_from date.",
            json!({
                "type": "object",
                "properties": {
                    "subject": {"type":"string","description":"The entity doing or being something"},
                    "predicate": {"type":"string","description":"Relationship type such as loves, works_on, or child_of"},
                    "object": {"type":"string","description":"The connected entity"},
                    "valid_from": {"type":"string","description":"When this fact became true (YYYY-MM-DD, optional)"}
                },
                "required": ["subject", "predicate", "object"]
            }),
        ),
        tool(
            "mempalace_kg_invalidate",
            "Mark a fact as no longer true by setting its end date.",
            json!({
                "type": "object",
                "properties": {
                    "subject": {"type":"string","description":"Entity"},
                    "predicate": {"type":"string","description":"Relationship"},
                    "object": {"type":"string","description":"Connected entity"},
                    "ended": {"type":"string","description":"When it stopped being true (YYYY-MM-DD, optional)"}
                },
                "required": ["subject", "predicate", "object"]
            }),
        ),
        tool(
            "mempalace_diary_write",
            "Write a timestamped diary entry for an agent with an optional topic.",
            json!({
                "type": "object",
                "properties": {
                    "agent_name": {"type":"string","description":"Agent name"},
                    "entry": {"type":"string","description":"Diary content"},
                    "topic": {"type":"string","description":"Topic label (default: general)"}
                },
                "required": ["agent_name", "entry"]
            }),
        ),
    ]
}
