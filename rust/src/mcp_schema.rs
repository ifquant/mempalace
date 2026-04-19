use serde_json::{Value, json};

use crate::error::{MempalaceError, Result};

pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 4] =
    ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

pub const PALACE_PROTOCOL: &str = "IMPORTANT — MemPalace Memory Protocol:\n1. ON WAKE-UP: Call mempalace_status to load palace overview + AAAK spec.\n2. BEFORE RESPONDING about any person, project, or past event: call mempalace_kg_query or mempalace_search FIRST. Never guess — verify.\n3. IF UNSURE about a fact (name, gender, age, relationship): say \"let me check\" and query the palace. Wrong is worse than slow.\n4. AFTER EACH SESSION: call mempalace_diary_write to record what happened, what you learned, what matters.\n5. WHEN FACTS CHANGE: call mempalace_kg_invalidate on the old fact, mempalace_kg_add for the new one.\n\nThis protocol ensures the AI KNOWS before it speaks. Storage is not memory — but storage + this protocol = memory.";

pub fn negotiate_protocol(params: Option<&Value>) -> &'static str {
    let requested = params
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str);

    match requested {
        Some(version) => SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .copied()
            .find(|supported| *supported == version)
            .unwrap_or(SUPPORTED_PROTOCOL_VERSIONS[0]),
        None => SUPPORTED_PROTOCOL_VERSIONS[1],
    }
}

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
            "mempalace_onboarding",
            "Bootstrap a project-local world model and registry files.",
            json!({
                "type": "object",
                "properties": {
                    "project_dir": {"type":"string","description":"Project directory to bootstrap"},
                    "mode": {"type":"string","description":"Onboarding mode: work, personal, or combo"},
                    "people": {"type":"array","items":{"type":"string"},"description":"People in name,relationship,context format"},
                    "projects": {"type":"array","items":{"type":"string"},"description":"Project names to seed"},
                    "aliases": {"type":"array","items":{"type":"string"},"description":"Alias mappings in alias=canonical format"},
                    "wings": {"type":"array","items":{"type":"string"},"description":"Wing names to seed"},
                    "scan": {"type":"boolean","description":"Scan local files for additional names"},
                    "auto_accept_detected": {"type":"boolean","description":"Auto-accept detected names during scan"}
                },
                "required": ["project_dir"]
            }),
        ),
        tool(
            "mempalace_normalize",
            "Normalize one chat export or transcript into MemPalace conversation format.",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type":"string","description":"Chat export or transcript file to normalize"}
                },
                "required": ["file_path"]
            }),
        ),
        tool(
            "mempalace_split",
            "Split transcript mega-files into per-session text files.",
            json!({
                "type": "object",
                "properties": {
                    "source_dir": {"type":"string","description":"Directory containing transcript files"},
                    "output_dir": {"type":"string","description":"Optional output directory for split files"},
                    "min_sessions": {"type":"integer","description":"Only split files with at least this many sessions (default 2)"},
                    "dry_run": {"type":"boolean","description":"Preview without writing files"}
                },
                "required": ["source_dir"]
            }),
        ),
        tool(
            "mempalace_instructions",
            "Return one built-in MemPalace instruction document.",
            json!({
                "type": "object",
                "properties": {
                    "name": {"type":"string","description":"Instruction set name: help, init, mine, search, or status"}
                },
                "required": ["name"]
            }),
        ),
        tool(
            "mempalace_hook_run",
            "Run one MemPalace hook using explicit MCP arguments instead of stdin.",
            json!({
                "type": "object",
                "properties": {
                    "hook": {"type":"string","description":"Hook name: session-start, stop, or precompact"},
                    "harness": {"type":"string","description":"Harness type: claude-code or codex"},
                    "session_id": {"type":"string","description":"Session identifier"},
                    "stop_hook_active": {"type":"boolean","description":"Whether the stop hook is already active"},
                    "transcript_path": {"type":"string","description":"Optional transcript JSONL path for stop counting"}
                },
                "required": ["hook", "harness"]
            }),
        ),
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

pub fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

pub fn requires_existing_palace(tool_name: &str) -> bool {
    !matches!(
        tool_name,
        "mempalace_diary_write"
            | "mempalace_add_drawer"
            | "mempalace_kg_add"
            | "mempalace_onboarding"
            | "mempalace_normalize"
            | "mempalace_split"
            | "mempalace_instructions"
            | "mempalace_hook_run"
            | "mempalace_registry_summary"
            | "mempalace_registry_lookup"
            | "mempalace_registry_query"
            | "mempalace_registry_learn"
            | "mempalace_registry_add_person"
            | "mempalace_registry_add_project"
            | "mempalace_registry_add_alias"
            | "mempalace_registry_research"
            | "mempalace_registry_confirm"
    )
}

pub fn no_palace() -> Value {
    json!({
        "error": "No palace found",
        "hint": "Run: mempalace init <dir> && mempalace mine <dir>",
    })
}

pub fn coerce_argument_types(tool_name: &str, arguments: &mut Value) {
    let Some(args) = arguments.as_object_mut() else {
        return;
    };

    match tool_name {
        "mempalace_search" | "mempalace_recall" => {
            coerce_u64(args, "limit");
        }
        "mempalace_check_duplicate" => {
            coerce_f64(args, "threshold");
        }
        "mempalace_dedup" => {
            coerce_f64(args, "threshold");
            coerce_bool(args, "dry_run");
            coerce_bool(args, "stats_only");
            coerce_u64(args, "min_count");
        }
        "mempalace_repair_prune" | "mempalace_compress" => {
            coerce_bool(args, "confirm");
            coerce_bool(args, "dry_run");
        }
        "mempalace_onboarding" => {
            coerce_bool(args, "scan");
            coerce_bool(args, "auto_accept_detected");
        }
        "mempalace_split" => {
            coerce_bool(args, "dry_run");
            coerce_u64(args, "min_sessions");
        }
        "mempalace_hook_run" => {
            coerce_bool(args, "stop_hook_active");
        }
        "mempalace_registry_research" => {
            coerce_bool(args, "auto_confirm");
        }
        "mempalace_traverse" => {
            coerce_u64(args, "max_hops");
        }
        "mempalace_diary_read" => {
            coerce_u64(args, "last_n");
        }
        "mempalace_kg_add"
        | "mempalace_kg_invalidate"
        | "mempalace_add_drawer"
        | "mempalace_delete_drawer" => {}
        _ => {}
    }
}

pub fn truncate_duplicate_content(text: &str) -> String {
    const PREVIEW_LIMIT: usize = 200;
    if text.chars().count() <= PREVIEW_LIMIT {
        text.to_string()
    } else {
        let preview = text.chars().take(PREVIEW_LIMIT).collect::<String>();
        format!("{preview}...")
    }
}

pub fn required_str<'a>(arguments: &'a Value, key: &str, tool_name: &str) -> Result<&'a str> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| MempalaceError::Mcp(format!("{tool_name} requires {key}")))
}

pub fn string_list_arg(arguments: &Value, key: &str) -> Vec<String> {
    match arguments.get(key) {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn coerce_bool(args: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = args.get(key).cloned() {
        let coerced = match value {
            Value::String(text) => text.parse::<bool>().ok().map(Value::from),
            Value::Bool(_) => Some(value),
            _ => None,
        };
        if let Some(flag) = coerced {
            args.insert(key.to_string(), flag);
        }
    }
}

fn coerce_u64(args: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = args.get(key).cloned() {
        let coerced = match value {
            Value::String(text) => text.parse::<u64>().ok().map(Value::from),
            Value::Number(_) => value.as_u64().map(Value::from),
            _ => None,
        };
        if let Some(number) = coerced {
            args.insert(key.to_string(), number);
        }
    }
}

fn coerce_f64(args: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = args.get(key).cloned() {
        let coerced = match value {
            Value::String(text) => text.parse::<f64>().ok().map(Value::from),
            Value::Number(_) => value.as_f64().map(Value::from),
            _ => None,
        };
        if let Some(number) = coerced {
            args.insert(key.to_string(), number);
        }
    }
}
