use serde_json::{Value, json};

use crate::audit::WriteAheadLog;
use crate::config::AppConfig;
use crate::dialect::AAAK_SPEC;
use crate::error::{MempalaceError, Result};
use crate::hook;
use crate::instructions;
use crate::normalize::normalize_conversation_file;
use crate::onboarding::{OnboardingRequest, parse_alias_arg, parse_person_arg, run_onboarding};
use crate::service::App;
use crate::split;

pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 4] =
    ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const PALACE_PROTOCOL: &str = "IMPORTANT — MemPalace Memory Protocol:\n1. ON WAKE-UP: Call mempalace_status to load palace overview + AAAK spec.\n2. BEFORE RESPONDING about any person, project, or past event: call mempalace_kg_query or mempalace_search FIRST. Never guess — verify.\n3. IF UNSURE about a fact (name, gender, age, relationship): say \"let me check\" and query the palace. Wrong is worse than slow.\n4. AFTER EACH SESSION: call mempalace_diary_write to record what happened, what you learned, what matters.\n5. WHEN FACTS CHANGE: call mempalace_kg_invalidate on the old fact, mempalace_kg_add for the new one.\n\nThis protocol ensures the AI KNOWS before it speaks. Storage is not memory — but storage + this protocol = memory.";

pub async fn handle_request(request: Value, config: &AppConfig) -> Result<Option<Value>> {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| MempalaceError::Mcp("missing method".to_string()))?;
    let id = request.get("id").cloned().unwrap_or(Value::Null);

    match method {
        "initialize" => {
            let protocol_version = negotiate_protocol(request.get("params"));
            Ok(Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": protocol_version,
                    "serverInfo": {
                        "name": "mempalace",
                        "version": crate::VERSION,
                    },
                    "capabilities": {
                        "tools": {}
                    }
                }
            })))
        }
        "notifications/initialized" => Ok(None),
        "tools/list" => Ok(Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tools()
            }
        }))),
        "tools/call" => {
            let name = request["params"]["name"]
                .as_str()
                .ok_or_else(|| MempalaceError::Mcp("missing tool name".to_string()))?;
            let mut arguments = request["params"]
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            coerce_argument_types(name, &mut arguments);
            match call_tool(name, arguments, config).await {
                Ok(result) => Ok(Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result)?,
                            }
                        ]
                    }
                }))),
                Err(err) => Ok(Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32000,
                        "message": err.to_string()
                    }
                }))),
            }
        }
        _ => Ok(Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Unknown method: {method}")
            }
        }))),
    }
}

pub async fn run_stdio(config: AppConfig) -> Result<()> {
    use std::io::{self, BufRead, Write};

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = serde_json::from_str(&line)?;
        if let Some(response) = handle_request(request, &config).await? {
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        }
    }
    Ok(())
}

fn negotiate_protocol(params: Option<&Value>) -> &'static str {
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

fn tools() -> Vec<Value> {
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

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

async fn call_tool(name: &str, arguments: Value, config: &AppConfig) -> Result<Value> {
    if requires_existing_palace(name) && !palace_exists(config) {
        return Ok(no_palace());
    }

    let app = App::new(config.clone())?;

    match name {
        "mempalace_status" => match app.status().await {
            Ok(status) => Ok(json!({
                "kind": status.kind,
                "total_drawers": status.total_drawers,
                "wings": status.wings,
                "rooms": status.rooms,
                "palace_path": status.palace_path,
                "sqlite_path": status.sqlite_path,
                "lance_path": status.lance_path,
                "version": status.version,
                "schema_version": status.schema_version,
                "protocol": PALACE_PROTOCOL,
                "aaak_dialect": AAAK_SPEC,
            })),
            Err(err) => Ok(tool_error(
                "Status error",
                &err,
                "Check the palace files, then rerun mempalace_status.",
            )),
        },
        "mempalace_list_wings" => match app.list_wings().await {
            Ok(wings) => Ok(json!({ "wings": wings })),
            Err(err) => Ok(tool_error(
                "List wings error",
                &err,
                "Check the palace files, then rerun mempalace_list_wings.",
            )),
        },
        "mempalace_list_rooms" => {
            let wing = arguments.get("wing").and_then(Value::as_str);
            match app.list_rooms(wing).await {
                Ok(rooms) => Ok(json!({
                    "wing": rooms.wing,
                    "rooms": rooms.rooms,
                })),
                Err(err) => Ok(tool_error(
                    "List rooms error",
                    &err,
                    "Check the palace files and wing filter, then rerun mempalace_list_rooms.",
                )),
            }
        }
        "mempalace_get_taxonomy" => match app.taxonomy().await {
            Ok(taxonomy) => Ok(serde_json::to_value(taxonomy)?),
            Err(err) => Ok(tool_error(
                "Taxonomy error",
                &err,
                "Check the palace files, then rerun mempalace_get_taxonomy.",
            )),
        },
        "mempalace_search" => {
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .ok_or_else(|| MempalaceError::Mcp("mempalace_search requires query".to_string()));
            let Ok(query) = query else {
                return Ok(tool_error(
                    "Search error",
                    &MempalaceError::Mcp("mempalace_search requires query".to_string()),
                    "Provide a query string, then rerun mempalace_search.",
                ));
            };
            let wing = arguments.get("wing").and_then(Value::as_str);
            let room = arguments.get("room").and_then(Value::as_str);
            let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize;
            let results = match app.search(query, wing, room, limit).await {
                Ok(results) => results,
                Err(err) => {
                    return Ok(tool_error(
                        "Search error",
                        &err,
                        "Check the query, embedding provider, and palace files, then rerun mempalace_search.",
                    ));
                }
            };
            Ok(json!({
                "query": results.query,
                "filters": results.filters,
                "results": results.results.into_iter().map(|hit| {
                    json!({
                        "text": hit.text,
                        "wing": hit.wing,
                        "room": hit.room,
                        "source_file": hit.source_file,
                        "similarity": hit.similarity,
                    })
                }).collect::<Vec<_>>()
            }))
        }
        "mempalace_check_duplicate" => {
            let content = arguments
                .get("content")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    MempalaceError::Mcp("mempalace_check_duplicate requires content".to_string())
                });
            let Ok(content) = content else {
                return Ok(tool_error(
                    "Check duplicate error",
                    &MempalaceError::Mcp("mempalace_check_duplicate requires content".to_string()),
                    "Provide content text, then rerun mempalace_check_duplicate.",
                ));
            };
            let threshold = arguments
                .get("threshold")
                .and_then(Value::as_f64)
                .unwrap_or(0.9);
            let results = match app.search(content, None, None, 5).await {
                Ok(results) => results,
                Err(err) => {
                    return Ok(tool_error(
                        "Check duplicate error",
                        &err,
                        "Check the content, embedding provider, and palace files, then rerun mempalace_check_duplicate.",
                    ));
                }
            };
            let matches = results
                .results
                .into_iter()
                .filter_map(|hit| {
                    let similarity = hit.similarity?;
                    if similarity < threshold {
                        return None;
                    }
                    Some(json!({
                        "id": hit.id,
                        "wing": hit.wing,
                        "room": hit.room,
                        "similarity": similarity,
                        "content": truncate_duplicate_content(&hit.text),
                    }))
                })
                .collect::<Vec<_>>();
            Ok(json!({
                "is_duplicate": !matches.is_empty(),
                "matches": matches,
            }))
        }
        "mempalace_add_drawer" => {
            let wing = required_str(&arguments, "wing", "mempalace_add_drawer");
            let Ok(wing) = wing else {
                return Ok(tool_error(
                    "Add drawer error",
                    &MempalaceError::Mcp("mempalace_add_drawer requires wing".to_string()),
                    "Provide wing, room, and content, then rerun mempalace_add_drawer.",
                ));
            };
            let room = required_str(&arguments, "room", "mempalace_add_drawer");
            let Ok(room) = room else {
                return Ok(tool_error(
                    "Add drawer error",
                    &MempalaceError::Mcp("mempalace_add_drawer requires room".to_string()),
                    "Provide wing, room, and content, then rerun mempalace_add_drawer.",
                ));
            };
            let content = required_str(&arguments, "content", "mempalace_add_drawer");
            let Ok(content) = content else {
                return Ok(tool_error(
                    "Add drawer error",
                    &MempalaceError::Mcp("mempalace_add_drawer requires content".to_string()),
                    "Provide wing, room, and content, then rerun mempalace_add_drawer.",
                ));
            };
            let source_file = arguments.get("source_file").and_then(Value::as_str);
            let added_by = arguments.get("added_by").and_then(Value::as_str);
            best_effort_wal_log(
                config,
                "add_drawer",
                json!({
                    "wing": wing,
                    "room": room,
                    "added_by": added_by.unwrap_or("mcp"),
                    "content_length": content.chars().count(),
                    "content_preview": truncate_duplicate_content(content),
                    "source_file": source_file.unwrap_or(""),
                }),
            );
            match app
                .add_drawer(wing, room, content, source_file, added_by)
                .await
            {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Add drawer error",
                    &err,
                    "Check wing, room, content, and palace files, then rerun mempalace_add_drawer.",
                )),
            }
        }
        "mempalace_delete_drawer" => {
            let drawer_id = required_str(&arguments, "drawer_id", "mempalace_delete_drawer");
            let Ok(drawer_id) = drawer_id else {
                return Ok(tool_error(
                    "Delete drawer error",
                    &MempalaceError::Mcp("mempalace_delete_drawer requires drawer_id".to_string()),
                    "Provide drawer_id, then rerun mempalace_delete_drawer.",
                ));
            };
            best_effort_wal_log(
                config,
                "delete_drawer",
                json!({
                    "drawer_id": drawer_id,
                }),
            );
            match app.delete_drawer(drawer_id).await {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Delete drawer error",
                    &err,
                    "Check the drawer_id and palace files, then rerun mempalace_delete_drawer.",
                )),
            }
        }
        "mempalace_get_aaak_spec" => Ok(json!({
            "aaak_spec": AAAK_SPEC,
        })),
        "mempalace_wake_up" => {
            let wing = arguments.get("wing").and_then(Value::as_str);
            match app.wake_up(wing).await {
                Ok(summary) => Ok(serde_json::to_value(summary)?),
                Err(err) => Ok(tool_error(
                    "Wake-up error",
                    &err,
                    "Check the palace files and optional wing filter, then rerun mempalace_wake_up.",
                )),
            }
        }
        "mempalace_recall" => {
            let wing = arguments.get("wing").and_then(Value::as_str);
            let room = arguments.get("room").and_then(Value::as_str);
            let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
            match app.recall(wing, room, limit).await {
                Ok(summary) => Ok(serde_json::to_value(summary)?),
                Err(err) => Ok(tool_error(
                    "Recall error",
                    &err,
                    "Check the palace files and wing/room filters, then rerun mempalace_recall.",
                )),
            }
        }
        "mempalace_layers_status" => match app.layer_status().await {
            Ok(summary) => Ok(serde_json::to_value(summary)?),
            Err(err) => Ok(tool_error(
                "Layers status error",
                &err,
                "Check the palace files, then rerun mempalace_layers_status.",
            )),
        },
        "mempalace_repair" => match app.repair().await {
            Ok(summary) => Ok(serde_json::to_value(summary)?),
            Err(err) => Ok(tool_error(
                "Repair error",
                &err,
                "Check the palace files, then rerun mempalace_repair.",
            )),
        },
        "mempalace_repair_scan" => {
            let wing = arguments.get("wing").and_then(Value::as_str);
            match app.repair_scan(wing).await {
                Ok(summary) => Ok(serde_json::to_value(summary)?),
                Err(err) => Ok(tool_error(
                    "Repair scan error",
                    &err,
                    "Check the palace files and optional wing filter, then rerun mempalace_repair_scan.",
                )),
            }
        }
        "mempalace_repair_prune" => {
            let confirm = arguments
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match app.repair_prune(confirm).await {
                Ok(summary) => Ok(serde_json::to_value(summary)?),
                Err(err) => Ok(tool_error(
                    "Repair prune error",
                    &err,
                    "Check corrupt_ids.txt and palace files, then rerun mempalace_repair_prune.",
                )),
            }
        }
        "mempalace_repair_rebuild" => match app.repair_rebuild().await {
            Ok(summary) => Ok(serde_json::to_value(summary)?),
            Err(err) => Ok(tool_error(
                "Repair rebuild error",
                &err,
                "Check the palace files, embedding profile, and vector store, then rerun mempalace_repair_rebuild.",
            )),
        },
        "mempalace_compress" => {
            let wing = arguments.get("wing").and_then(Value::as_str);
            let dry_run = arguments
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match app.compress(wing, dry_run).await {
                Ok(summary) => Ok(serde_json::to_value(summary)?),
                Err(err) => Ok(tool_error(
                    "Compress error",
                    &err,
                    "Check the palace files and optional wing filter, then rerun mempalace_compress.",
                )),
            }
        }
        "mempalace_dedup" => {
            let threshold = arguments
                .get("threshold")
                .and_then(Value::as_f64)
                .unwrap_or(0.15);
            let dry_run = arguments
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let stats_only = arguments
                .get("stats_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let wing = arguments.get("wing").and_then(Value::as_str);
            let source = arguments.get("source").and_then(Value::as_str);
            let min_count = arguments
                .get("min_count")
                .and_then(Value::as_u64)
                .unwrap_or(5) as usize;
            match app
                .dedup(threshold, dry_run, wing, source, min_count, stats_only)
                .await
            {
                Ok(summary) => Ok(serde_json::to_value(summary)?),
                Err(err) => Ok(tool_error(
                    "Dedup error",
                    &err,
                    "Check the palace files and dedup filters, then rerun mempalace_dedup.",
                )),
            }
        }
        "mempalace_onboarding" => {
            let project_dir = required_str(&arguments, "project_dir", "mempalace_onboarding");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Onboarding error",
                    &MempalaceError::Mcp("mempalace_onboarding requires project_dir".to_string()),
                    "Provide project_dir, then rerun mempalace_onboarding.",
                ));
            };
            let mut request = OnboardingRequest {
                mode: arguments
                    .get("mode")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                people: Vec::new(),
                projects: string_list_arg(&arguments, "projects"),
                aliases: std::collections::BTreeMap::new(),
                wings: string_list_arg(&arguments, "wings"),
                scan: arguments.get("scan").and_then(Value::as_bool),
                auto_accept_detected: arguments
                    .get("auto_accept_detected")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            };

            for person in string_list_arg(&arguments, "people") {
                match parse_person_arg(&person) {
                    Ok(value) => request.people.push(value),
                    Err(err) => {
                        return Ok(tool_error(
                            "Onboarding error",
                            &err,
                            "Use people entries in name,relationship,context format, then rerun mempalace_onboarding.",
                        ));
                    }
                }
            }
            for alias in string_list_arg(&arguments, "aliases") {
                match parse_alias_arg(&alias) {
                    Ok((alias, canonical)) => {
                        request.aliases.insert(alias, canonical);
                    }
                    Err(err) => {
                        return Ok(tool_error(
                            "Onboarding error",
                            &err,
                            "Use aliases in alias=canonical format, then rerun mempalace_onboarding.",
                        ));
                    }
                }
            }

            match run_onboarding(std::path::Path::new(project_dir), request) {
                Ok(summary) => Ok(serde_json::to_value(summary)?),
                Err(err) => Ok(tool_error(
                    "Onboarding error",
                    &err,
                    "Check project_dir and onboarding inputs, then rerun mempalace_onboarding.",
                )),
            }
        }
        "mempalace_normalize" => {
            let file_path = required_str(&arguments, "file_path", "mempalace_normalize");
            let Ok(file_path) = file_path else {
                return Ok(tool_error(
                    "Normalize error",
                    &MempalaceError::Mcp("mempalace_normalize requires file_path".to_string()),
                    "Provide file_path, then rerun mempalace_normalize.",
                ));
            };
            let path = std::path::Path::new(file_path);
            let raw = match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(err) => {
                    return Ok(tool_error(
                        "Normalize error",
                        &err,
                        "Check file_path and file readability, then rerun mempalace_normalize.",
                    ));
                }
            };
            match normalize_conversation_file(path) {
                Ok(Some(normalized)) => Ok(json!({
                    "kind": "normalize",
                    "file_path": path.display().to_string(),
                    "changed": normalized != raw,
                    "chars": normalized.chars().count(),
                    "quote_turns": normalized.lines().filter(|line| line.trim_start().starts_with('>')).count(),
                    "normalized": normalized,
                })),
                Ok(None) => Ok(tool_error(
                    "Normalize error",
                    &MempalaceError::InvalidArgument(
                        "Unsupported or unreadable conversation file.".to_string(),
                    ),
                    "Use a supported .txt, .md, .json, or .jsonl chat export, then rerun mempalace_normalize.",
                )),
                Err(err) => Ok(tool_error(
                    "Normalize error",
                    &err,
                    "Check file_path and transcript format, then rerun mempalace_normalize.",
                )),
            }
        }
        "mempalace_split" => {
            let source_dir = required_str(&arguments, "source_dir", "mempalace_split");
            let Ok(source_dir) = source_dir else {
                return Ok(tool_error(
                    "Split error",
                    &MempalaceError::Mcp("mempalace_split requires source_dir".to_string()),
                    "Provide source_dir, then rerun mempalace_split.",
                ));
            };
            let output_dir = arguments.get("output_dir").and_then(Value::as_str);
            let min_sessions = arguments
                .get("min_sessions")
                .and_then(Value::as_u64)
                .unwrap_or(2) as usize;
            let dry_run = arguments
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match split::split_directory(
                std::path::Path::new(source_dir),
                output_dir.map(std::path::Path::new),
                min_sessions,
                dry_run,
            ) {
                Ok(summary) => Ok(serde_json::to_value(summary)?),
                Err(err) => Ok(tool_error(
                    "Split error",
                    &err,
                    "Check source_dir, output_dir, and transcript files, then rerun mempalace_split.",
                )),
            }
        }
        "mempalace_instructions" => {
            let name = required_str(&arguments, "name", "mempalace_instructions");
            let Ok(name) = name else {
                return Ok(tool_error(
                    "Instructions error",
                    &MempalaceError::Mcp("mempalace_instructions requires name".to_string()),
                    "Provide an instruction name, then rerun mempalace_instructions.",
                ));
            };
            match instructions::render(name) {
                Ok(text) => Ok(json!({
                    "kind": "instructions",
                    "name": name,
                    "text": text,
                })),
                Err(err) => Ok(tool_error(
                    "Instructions error",
                    &err,
                    "Use one of help, init, mine, search, or status, then rerun mempalace_instructions.",
                )),
            }
        }
        "mempalace_hook_run" => {
            let hook_name = required_str(&arguments, "hook", "mempalace_hook_run");
            let Ok(hook_name) = hook_name else {
                return Ok(tool_error(
                    "Hook run error",
                    &MempalaceError::Mcp("mempalace_hook_run requires hook".to_string()),
                    "Provide hook and harness, then rerun mempalace_hook_run.",
                ));
            };
            let harness = required_str(&arguments, "harness", "mempalace_hook_run");
            let Ok(harness) = harness else {
                return Ok(tool_error(
                    "Hook run error",
                    &MempalaceError::Mcp("mempalace_hook_run requires harness".to_string()),
                    "Provide hook and harness, then rerun mempalace_hook_run.",
                ));
            };
            let payload = json!({
                "session_id": arguments.get("session_id").and_then(Value::as_str).unwrap_or("unknown"),
                "stop_hook_active": arguments.get("stop_hook_active").and_then(Value::as_bool).unwrap_or(false),
                "transcript_path": arguments.get("transcript_path").and_then(Value::as_str).unwrap_or_default(),
            });
            match hook::run_hook_with_data(hook_name, harness, &payload, config) {
                Ok(result) => Ok(json!({
                    "kind": "hook_run",
                    "hook": hook_name,
                    "harness": harness,
                    "result": result,
                })),
                Err(err) => Ok(tool_error(
                    "Hook run error",
                    &err,
                    "Check hook, harness, and transcript_path, then rerun mempalace_hook_run.",
                )),
            }
        }
        "mempalace_registry_summary" => {
            let project_dir = required_str(&arguments, "project_dir", "mempalace_registry_summary");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry summary error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_summary requires project_dir".to_string(),
                    ),
                    "Provide project_dir, then rerun mempalace_registry_summary.",
                ));
            };
            match app.registry_summary(std::path::Path::new(project_dir)) {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Registry summary error",
                    &err,
                    "Check project_dir and entity_registry.json, then rerun mempalace_registry_summary.",
                )),
            }
        }
        "mempalace_registry_lookup" => {
            let project_dir = required_str(&arguments, "project_dir", "mempalace_registry_lookup");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry lookup error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_lookup requires project_dir".to_string(),
                    ),
                    "Provide project_dir and word, then rerun mempalace_registry_lookup.",
                ));
            };
            let word = required_str(&arguments, "word", "mempalace_registry_lookup");
            let Ok(word) = word else {
                return Ok(tool_error(
                    "Registry lookup error",
                    &MempalaceError::Mcp("mempalace_registry_lookup requires word".to_string()),
                    "Provide project_dir and word, then rerun mempalace_registry_lookup.",
                ));
            };
            let context = arguments
                .get("context")
                .and_then(Value::as_str)
                .unwrap_or("");
            match app.registry_lookup(std::path::Path::new(project_dir), word, context) {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Registry lookup error",
                    &err,
                    "Check project_dir, word, and entity_registry.json, then rerun mempalace_registry_lookup.",
                )),
            }
        }
        "mempalace_registry_query" => {
            let project_dir = required_str(&arguments, "project_dir", "mempalace_registry_query");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry query error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_query requires project_dir".to_string(),
                    ),
                    "Provide project_dir and query, then rerun mempalace_registry_query.",
                ));
            };
            let query = required_str(&arguments, "query", "mempalace_registry_query");
            let Ok(query) = query else {
                return Ok(tool_error(
                    "Registry query error",
                    &MempalaceError::Mcp("mempalace_registry_query requires query".to_string()),
                    "Provide project_dir and query, then rerun mempalace_registry_query.",
                ));
            };
            match app.registry_query(std::path::Path::new(project_dir), query) {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Registry query error",
                    &err,
                    "Check project_dir, query, and entity_registry.json, then rerun mempalace_registry_query.",
                )),
            }
        }
        "mempalace_registry_learn" => {
            let project_dir = required_str(&arguments, "project_dir", "mempalace_registry_learn");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry learn error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_learn requires project_dir".to_string(),
                    ),
                    "Provide project_dir, then rerun mempalace_registry_learn.",
                ));
            };
            match app.registry_learn(std::path::Path::new(project_dir)) {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Registry learn error",
                    &err,
                    "Check project_dir and entity_registry.json, then rerun mempalace_registry_learn.",
                )),
            }
        }
        "mempalace_registry_add_person" => {
            let project_dir =
                required_str(&arguments, "project_dir", "mempalace_registry_add_person");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry add person error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_add_person requires project_dir".to_string(),
                    ),
                    "Provide project_dir and name, then rerun mempalace_registry_add_person.",
                ));
            };
            let name = required_str(&arguments, "name", "mempalace_registry_add_person");
            let Ok(name) = name else {
                return Ok(tool_error(
                    "Registry add person error",
                    &MempalaceError::Mcp("mempalace_registry_add_person requires name".to_string()),
                    "Provide project_dir and name, then rerun mempalace_registry_add_person.",
                ));
            };
            let relationship = arguments
                .get("relationship")
                .and_then(Value::as_str)
                .unwrap_or("");
            let context = arguments
                .get("context")
                .and_then(Value::as_str)
                .unwrap_or("work");
            best_effort_wal_log(
                config,
                "registry_add_person",
                json!({
                    "project_dir": project_dir,
                    "name": name,
                    "relationship": relationship,
                    "context": context,
                }),
            );
            match app.registry_add_person(
                std::path::Path::new(project_dir),
                name,
                relationship,
                context,
            ) {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Registry add person error",
                    &err,
                    "Check project_dir, name, and entity_registry.json, then rerun mempalace_registry_add_person.",
                )),
            }
        }
        "mempalace_registry_add_project" => {
            let project_dir =
                required_str(&arguments, "project_dir", "mempalace_registry_add_project");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry add project error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_add_project requires project_dir".to_string(),
                    ),
                    "Provide project_dir and name, then rerun mempalace_registry_add_project.",
                ));
            };
            let name = required_str(&arguments, "name", "mempalace_registry_add_project");
            let Ok(name) = name else {
                return Ok(tool_error(
                    "Registry add project error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_add_project requires name".to_string(),
                    ),
                    "Provide project_dir and name, then rerun mempalace_registry_add_project.",
                ));
            };
            best_effort_wal_log(
                config,
                "registry_add_project",
                json!({
                    "project_dir": project_dir,
                    "name": name,
                }),
            );
            match app.registry_add_project(std::path::Path::new(project_dir), name) {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Registry add project error",
                    &err,
                    "Check project_dir, name, and entity_registry.json, then rerun mempalace_registry_add_project.",
                )),
            }
        }
        "mempalace_registry_add_alias" => {
            let project_dir =
                required_str(&arguments, "project_dir", "mempalace_registry_add_alias");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry add alias error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_add_alias requires project_dir".to_string(),
                    ),
                    "Provide project_dir, canonical, and alias, then rerun mempalace_registry_add_alias.",
                ));
            };
            let canonical = required_str(&arguments, "canonical", "mempalace_registry_add_alias");
            let Ok(canonical) = canonical else {
                return Ok(tool_error(
                    "Registry add alias error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_add_alias requires canonical".to_string(),
                    ),
                    "Provide project_dir, canonical, and alias, then rerun mempalace_registry_add_alias.",
                ));
            };
            let alias = required_str(&arguments, "alias", "mempalace_registry_add_alias");
            let Ok(alias) = alias else {
                return Ok(tool_error(
                    "Registry add alias error",
                    &MempalaceError::Mcp("mempalace_registry_add_alias requires alias".to_string()),
                    "Provide project_dir, canonical, and alias, then rerun mempalace_registry_add_alias.",
                ));
            };
            best_effort_wal_log(
                config,
                "registry_add_alias",
                json!({
                    "project_dir": project_dir,
                    "canonical": canonical,
                    "alias": alias,
                }),
            );
            match app.registry_add_alias(std::path::Path::new(project_dir), canonical, alias) {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Registry add alias error",
                    &err,
                    "Check project_dir, canonical, alias, and entity_registry.json, then rerun mempalace_registry_add_alias.",
                )),
            }
        }
        "mempalace_registry_research" => {
            let project_dir =
                required_str(&arguments, "project_dir", "mempalace_registry_research");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry research error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_research requires project_dir".to_string(),
                    ),
                    "Provide project_dir and word, then rerun mempalace_registry_research.",
                ));
            };
            let word = required_str(&arguments, "word", "mempalace_registry_research");
            let Ok(word) = word else {
                return Ok(tool_error(
                    "Registry research error",
                    &MempalaceError::Mcp("mempalace_registry_research requires word".to_string()),
                    "Provide project_dir and word, then rerun mempalace_registry_research.",
                ));
            };
            let auto_confirm = arguments
                .get("auto_confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            best_effort_wal_log(
                config,
                "registry_research",
                json!({
                    "project_dir": project_dir,
                    "word": word,
                    "auto_confirm": auto_confirm,
                }),
            );
            match app.registry_research(std::path::Path::new(project_dir), word, auto_confirm) {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Registry research error",
                    &err,
                    "Check project_dir, word, network access, and entity_registry.json, then rerun mempalace_registry_research.",
                )),
            }
        }
        "mempalace_registry_confirm" => {
            let project_dir = required_str(&arguments, "project_dir", "mempalace_registry_confirm");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry confirm error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_confirm requires project_dir".to_string(),
                    ),
                    "Provide project_dir and word, then rerun mempalace_registry_confirm.",
                ));
            };
            let word = required_str(&arguments, "word", "mempalace_registry_confirm");
            let Ok(word) = word else {
                return Ok(tool_error(
                    "Registry confirm error",
                    &MempalaceError::Mcp("mempalace_registry_confirm requires word".to_string()),
                    "Provide project_dir and word, then rerun mempalace_registry_confirm.",
                ));
            };
            let entity_type = arguments
                .get("entity_type")
                .and_then(Value::as_str)
                .unwrap_or("person");
            let relationship = arguments
                .get("relationship")
                .and_then(Value::as_str)
                .unwrap_or("");
            let context = arguments
                .get("context")
                .and_then(Value::as_str)
                .unwrap_or("personal");
            best_effort_wal_log(
                config,
                "registry_confirm",
                json!({
                    "project_dir": project_dir,
                    "word": word,
                    "entity_type": entity_type,
                    "relationship": relationship,
                    "context": context,
                }),
            );
            match app.registry_confirm_research(
                std::path::Path::new(project_dir),
                word,
                entity_type,
                relationship,
                context,
            ) {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Registry confirm error",
                    &err,
                    "Check project_dir, word, and entity_registry.json, then rerun mempalace_registry_confirm.",
                )),
            }
        }
        "mempalace_kg_query" => {
            let entity = arguments
                .get("entity")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    MempalaceError::Mcp("mempalace_kg_query requires entity".to_string())
                });
            let Ok(entity) = entity else {
                return Ok(tool_error(
                    "KG query error",
                    &MempalaceError::Mcp("mempalace_kg_query requires entity".to_string()),
                    "Provide an entity value, then rerun mempalace_kg_query.",
                ));
            };
            let as_of = arguments.get("as_of").and_then(Value::as_str);
            let direction = arguments
                .get("direction")
                .and_then(Value::as_str)
                .unwrap_or("both");
            if !matches!(direction, "outgoing" | "incoming" | "both") {
                return Ok(tool_error(
                    "KG query error",
                    &MempalaceError::Mcp(format!("unsupported direction: {direction}")),
                    "Use direction=outgoing, incoming, or both, then rerun mempalace_kg_query.",
                ));
            }
            match app.kg_query(entity, as_of, direction).await {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "KG query error",
                    &err,
                    "Check the palace files and query inputs, then rerun mempalace_kg_query.",
                )),
            }
        }
        "mempalace_kg_add" => {
            let subject = required_str(&arguments, "subject", "mempalace_kg_add");
            let Ok(subject) = subject else {
                return Ok(tool_error(
                    "KG add error",
                    &MempalaceError::Mcp("mempalace_kg_add requires subject".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_add.",
                ));
            };
            let predicate = required_str(&arguments, "predicate", "mempalace_kg_add");
            let Ok(predicate) = predicate else {
                return Ok(tool_error(
                    "KG add error",
                    &MempalaceError::Mcp("mempalace_kg_add requires predicate".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_add.",
                ));
            };
            let object = required_str(&arguments, "object", "mempalace_kg_add");
            let Ok(object) = object else {
                return Ok(tool_error(
                    "KG add error",
                    &MempalaceError::Mcp("mempalace_kg_add requires object".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_add.",
                ));
            };
            let valid_from = arguments.get("valid_from").and_then(Value::as_str);
            best_effort_wal_log(
                config,
                "kg_add",
                json!({
                    "subject": subject,
                    "predicate": predicate,
                    "object": object,
                    "valid_from": valid_from,
                }),
            );
            match app.kg_add(subject, predicate, object, valid_from).await {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "KG add error",
                    &err,
                    "Check the fact fields and palace files, then rerun mempalace_kg_add.",
                )),
            }
        }
        "mempalace_kg_invalidate" => {
            let subject = required_str(&arguments, "subject", "mempalace_kg_invalidate");
            let Ok(subject) = subject else {
                return Ok(tool_error(
                    "KG invalidate error",
                    &MempalaceError::Mcp("mempalace_kg_invalidate requires subject".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_invalidate.",
                ));
            };
            let predicate = required_str(&arguments, "predicate", "mempalace_kg_invalidate");
            let Ok(predicate) = predicate else {
                return Ok(tool_error(
                    "KG invalidate error",
                    &MempalaceError::Mcp("mempalace_kg_invalidate requires predicate".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_invalidate.",
                ));
            };
            let object = required_str(&arguments, "object", "mempalace_kg_invalidate");
            let Ok(object) = object else {
                return Ok(tool_error(
                    "KG invalidate error",
                    &MempalaceError::Mcp("mempalace_kg_invalidate requires object".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_invalidate.",
                ));
            };
            let ended = arguments.get("ended").and_then(Value::as_str);
            best_effort_wal_log(
                config,
                "kg_invalidate",
                json!({
                    "subject": subject,
                    "predicate": predicate,
                    "object": object,
                    "ended": ended,
                }),
            );
            match app.kg_invalidate(subject, predicate, object, ended).await {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "KG invalidate error",
                    &err,
                    "Check the fact fields and palace files, then rerun mempalace_kg_invalidate.",
                )),
            }
        }
        "mempalace_kg_timeline" => {
            let entity = arguments.get("entity").and_then(Value::as_str);
            match app.kg_timeline(entity).await {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "KG timeline error",
                    &err,
                    "Check the palace files, then rerun mempalace_kg_timeline.",
                )),
            }
        }
        "mempalace_kg_stats" => match app.kg_stats().await {
            Ok(result) => Ok(serde_json::to_value(result)?),
            Err(err) => Ok(tool_error(
                "KG stats error",
                &err,
                "Check the palace files, then rerun mempalace_kg_stats.",
            )),
        },
        "mempalace_diary_write" => {
            let agent_name = arguments
                .get("agent_name")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    MempalaceError::Mcp("mempalace_diary_write requires agent_name".to_string())
                });
            let Ok(agent_name) = agent_name else {
                return Ok(tool_error(
                    "Diary write error",
                    &MempalaceError::Mcp("mempalace_diary_write requires agent_name".to_string()),
                    "Provide agent_name and entry, then rerun mempalace_diary_write.",
                ));
            };
            let entry = arguments
                .get("entry")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    MempalaceError::Mcp("mempalace_diary_write requires entry".to_string())
                });
            let Ok(entry) = entry else {
                return Ok(tool_error(
                    "Diary write error",
                    &MempalaceError::Mcp("mempalace_diary_write requires entry".to_string()),
                    "Provide agent_name and entry, then rerun mempalace_diary_write.",
                ));
            };
            let topic = arguments
                .get("topic")
                .and_then(Value::as_str)
                .unwrap_or("general");
            best_effort_wal_log(
                config,
                "diary_write",
                json!({
                    "agent_name": agent_name,
                    "topic": topic,
                    "entry_length": entry.chars().count(),
                    "entry_preview": truncate_duplicate_content(entry),
                }),
            );
            match app.diary_write(agent_name, entry, topic).await {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Diary write error",
                    &err,
                    "Check the palace path and diary inputs, then rerun mempalace_diary_write.",
                )),
            }
        }
        "mempalace_diary_read" => {
            let agent_name = arguments
                .get("agent_name")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    MempalaceError::Mcp("mempalace_diary_read requires agent_name".to_string())
                });
            let Ok(agent_name) = agent_name else {
                return Ok(tool_error(
                    "Diary read error",
                    &MempalaceError::Mcp("mempalace_diary_read requires agent_name".to_string()),
                    "Provide agent_name, then rerun mempalace_diary_read.",
                ));
            };
            let last_n = arguments
                .get("last_n")
                .and_then(Value::as_u64)
                .unwrap_or(10) as usize;
            match app.diary_read(agent_name, last_n).await {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Diary read error",
                    &err,
                    "Check the palace path and agent name, then rerun mempalace_diary_read.",
                )),
            }
        }
        "mempalace_traverse" => {
            let start_room = arguments
                .get("start_room")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    MempalaceError::Mcp("mempalace_traverse requires start_room".to_string())
                });
            let Ok(start_room) = start_room else {
                return Ok(tool_error(
                    "Traverse error",
                    &MempalaceError::Mcp("mempalace_traverse requires start_room".to_string()),
                    "Provide a start_room value, then rerun mempalace_traverse.",
                ));
            };
            let max_hops = arguments
                .get("max_hops")
                .and_then(Value::as_u64)
                .unwrap_or(2) as usize;
            match app.traverse_graph(start_room, max_hops).await {
                Ok(result) => Ok(serde_json::to_value(result)?),
                Err(err) => Ok(tool_error(
                    "Traverse error",
                    &err,
                    "Check the palace files and room name, then rerun mempalace_traverse.",
                )),
            }
        }
        "mempalace_find_tunnels" => {
            let wing_a = arguments.get("wing_a").and_then(Value::as_str);
            let wing_b = arguments.get("wing_b").and_then(Value::as_str);
            match app.find_tunnels(wing_a, wing_b).await {
                Ok(tunnels) => Ok(serde_json::to_value(tunnels)?),
                Err(err) => Ok(tool_error(
                    "Find tunnels error",
                    &err,
                    "Check the palace files and wing filters, then rerun mempalace_find_tunnels.",
                )),
            }
        }
        "mempalace_graph_stats" => match app.graph_stats().await {
            Ok(stats) => Ok(serde_json::to_value(stats)?),
            Err(err) => Ok(tool_error(
                "Graph stats error",
                &err,
                "Check the palace files, then rerun mempalace_graph_stats.",
            )),
        },
        _ => Ok(json!({
            "error": {
                "code": -32601,
                "message": format!("Unknown tool: {name}")
            }
        })),
    }
}

fn tool_error(prefix: &str, err: &dyn std::fmt::Display, hint: &str) -> Value {
    json!({
        "error": format!("{prefix}: {err}"),
        "hint": hint,
    })
}

fn palace_exists(config: &AppConfig) -> bool {
    config.sqlite_path().exists() || config.lance_path().exists()
}

fn requires_existing_palace(tool_name: &str) -> bool {
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

fn no_palace() -> Value {
    json!({
        "error": "No palace found",
        "hint": "Run: mempalace init <dir> && mempalace mine <dir>",
    })
}

fn coerce_argument_types(tool_name: &str, arguments: &mut Value) {
    let Some(args) = arguments.as_object_mut() else {
        return;
    };

    match tool_name {
        "mempalace_search" => {
            if let Some(value) = args.get("limit").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<u64>().ok().map(Value::from),
                    Value::Number(_) => value.as_u64().map(Value::from),
                    _ => None,
                };
                if let Some(limit) = coerced {
                    args.insert("limit".to_string(), limit);
                }
            }
        }
        "mempalace_check_duplicate" => {
            if let Some(value) = args.get("threshold").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<f64>().ok().map(Value::from),
                    Value::Number(_) => value.as_f64().map(Value::from),
                    _ => None,
                };
                if let Some(threshold) = coerced {
                    args.insert("threshold".to_string(), threshold);
                }
            }
        }
        "mempalace_recall" => {
            if let Some(value) = args.get("limit").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<u64>().ok().map(Value::from),
                    Value::Number(_) => value.as_u64().map(Value::from),
                    _ => None,
                };
                if let Some(limit) = coerced {
                    args.insert("limit".to_string(), limit);
                }
            }
        }
        "mempalace_repair_prune" | "mempalace_compress" => {
            if let Some(value) = args.get("confirm").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<bool>().ok().map(Value::from),
                    Value::Bool(_) => Some(value),
                    _ => None,
                };
                if let Some(confirm) = coerced {
                    args.insert("confirm".to_string(), confirm);
                }
            }
            if let Some(value) = args.get("dry_run").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<bool>().ok().map(Value::from),
                    Value::Bool(_) => Some(value),
                    _ => None,
                };
                if let Some(dry_run) = coerced {
                    args.insert("dry_run".to_string(), dry_run);
                }
            }
        }
        "mempalace_dedup" => {
            if let Some(value) = args.get("threshold").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<f64>().ok().map(Value::from),
                    Value::Number(_) => value.as_f64().map(Value::from),
                    _ => None,
                };
                if let Some(threshold) = coerced {
                    args.insert("threshold".to_string(), threshold);
                }
            }
            for key in ["dry_run", "stats_only"] {
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
            if let Some(value) = args.get("min_count").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<u64>().ok().map(Value::from),
                    Value::Number(_) => value.as_u64().map(Value::from),
                    _ => None,
                };
                if let Some(min_count) = coerced {
                    args.insert("min_count".to_string(), min_count);
                }
            }
        }
        "mempalace_onboarding" => {
            for key in ["scan", "auto_accept_detected"] {
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
        }
        "mempalace_split" => {
            if let Some(value) = args.get("dry_run").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<bool>().ok().map(Value::from),
                    Value::Bool(_) => Some(value),
                    _ => None,
                };
                if let Some(dry_run) = coerced {
                    args.insert("dry_run".to_string(), dry_run);
                }
            }
            if let Some(value) = args.get("min_sessions").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<u64>().ok().map(Value::from),
                    Value::Number(_) => value.as_u64().map(Value::from),
                    _ => None,
                };
                if let Some(min_sessions) = coerced {
                    args.insert("min_sessions".to_string(), min_sessions);
                }
            }
        }
        "mempalace_hook_run" => {
            if let Some(value) = args.get("stop_hook_active").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<bool>().ok().map(Value::from),
                    Value::Bool(_) => Some(value),
                    _ => None,
                };
                if let Some(flag) = coerced {
                    args.insert("stop_hook_active".to_string(), flag);
                }
            }
        }
        "mempalace_kg_add"
        | "mempalace_kg_invalidate"
        | "mempalace_add_drawer"
        | "mempalace_delete_drawer" => {}
        "mempalace_registry_research" => {
            if let Some(value) = args.get("auto_confirm").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<bool>().ok().map(Value::from),
                    Value::Bool(_) => Some(value),
                    _ => None,
                };
                if let Some(auto_confirm) = coerced {
                    args.insert("auto_confirm".to_string(), auto_confirm);
                }
            }
        }
        "mempalace_traverse" => {
            if let Some(value) = args.get("max_hops").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<u64>().ok().map(Value::from),
                    Value::Number(_) => value.as_u64().map(Value::from),
                    _ => None,
                };
                if let Some(max_hops) = coerced {
                    args.insert("max_hops".to_string(), max_hops);
                }
            }
        }
        "mempalace_diary_read" => {
            if let Some(value) = args.get("last_n").cloned() {
                let coerced = match value {
                    Value::String(text) => text.parse::<u64>().ok().map(Value::from),
                    Value::Number(_) => value.as_u64().map(Value::from),
                    _ => None,
                };
                if let Some(last_n) = coerced {
                    args.insert("last_n".to_string(), last_n);
                }
            }
        }
        _ => {}
    }
}

fn truncate_duplicate_content(text: &str) -> String {
    const PREVIEW_LIMIT: usize = 200;
    if text.chars().count() <= PREVIEW_LIMIT {
        text.to_string()
    } else {
        let preview = text.chars().take(PREVIEW_LIMIT).collect::<String>();
        format!("{preview}...")
    }
}

fn required_str<'a>(arguments: &'a Value, key: &str, tool_name: &str) -> Result<&'a str> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| MempalaceError::Mcp(format!("{tool_name} requires {key}")))
}

fn string_list_arg(arguments: &Value, key: &str) -> Vec<String> {
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

fn best_effort_wal_log(config: &AppConfig, operation: &str, params: Value) {
    match WriteAheadLog::for_palace(&config.palace_path) {
        Ok(wal) => {
            if let Err(err) = wal.log(operation, params, None) {
                tracing::error!("WAL write failed: {err}");
            }
        }
        Err(err) => tracing::error!("WAL init failed: {err}"),
    }
}
