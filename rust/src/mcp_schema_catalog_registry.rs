use serde_json::{Value, json};

use crate::mcp_schema::tool;

pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "mempalace_registry_summary",
            "Summarize one project-local entity registry.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory containing entity_registry.json"}
                },
                "required": ["project_dir"]
            }),
        ),
        tool(
            "mempalace_registry_lookup",
            "Look up one word in a project-local entity registry.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory containing entity_registry.json"},
                    "word": {"type":"string","description":"Word to classify"},
                    "context": {"type":"string","description":"Optional surrounding sentence for disambiguation"}
                },
                "required": ["project_dir", "word"]
            }),
        ),
        tool(
            "mempalace_registry_query",
            "Extract known people and unknown candidates from a free-form query using a project-local registry.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory containing entity_registry.json"},
                    "query": {"type":"string","description":"Free-form query text"}
                },
                "required": ["project_dir", "query"]
            }),
        ),
        tool(
            "mempalace_registry_learn",
            "Learn new people and projects into a project-local entity registry from local files.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory containing entity_registry.json"}
                },
                "required": ["project_dir"]
            }),
        ),
        tool(
            "mempalace_registry_add_person",
            "Add one person to a project-local entity registry.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory containing entity_registry.json"},
                    "name": {"type":"string","description":"Person name"},
                    "relationship": {"type":"string","description":"Relationship or role"},
                    "context": {"type":"string","description":"Context bucket: work or personal"}
                },
                "required": ["project_dir", "name"]
            }),
        ),
        tool(
            "mempalace_registry_add_project",
            "Add one project name to a project-local entity registry.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory containing entity_registry.json"},
                    "name": {"type":"string","description":"Project name"}
                },
                "required": ["project_dir", "name"]
            }),
        ),
        tool(
            "mempalace_registry_add_alias",
            "Add an alias or nickname for an existing canonical person.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory containing entity_registry.json"},
                    "canonical": {"type":"string","description":"Canonical person name"},
                    "alias": {"type":"string","description":"Alias or nickname"}
                },
                "required": ["project_dir", "canonical", "alias"]
            }),
        ),
        tool(
            "mempalace_registry_research",
            "Research one word into the project-local registry wiki cache.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory containing entity_registry.json"},
                    "word": {"type":"string","description":"Word to research"},
                    "auto_confirm": {"type":"boolean","description":"Mark the research result confirmed immediately"}
                },
                "required": ["project_dir", "word"]
            }),
        ),
        tool(
            "mempalace_registry_confirm",
            "Confirm one researched word and promote it into the project-local registry.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory containing entity_registry.json"},
                    "word": {"type":"string","description":"Word already present in wiki_cache"},
                    "entity_type": {"type":"string","description":"Usually person"},
                    "relationship": {"type":"string","description":"Relationship or role"},
                    "context": {"type":"string","description":"Context bucket: work or personal"}
                },
                "required": ["project_dir", "word"]
            }),
        ),
    ]
}
