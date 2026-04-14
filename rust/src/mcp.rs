use serde_json::{Value, json};

use crate::config::AppConfig;
use crate::error::{MempalaceError, Result};
use crate::service::App;

pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 4] =
    ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const PALACE_PROTOCOL: &str = "IMPORTANT — MemPalace Memory Protocol:\n1. ON WAKE-UP: Call mempalace_status to load palace overview + AAAK spec.\n2. BEFORE RESPONDING about any person, project, or past event: call mempalace_kg_query or mempalace_search FIRST. Never guess — verify.\n3. IF UNSURE about a fact (name, gender, age, relationship): say \"let me check\" and query the palace. Wrong is worse than slow.\n4. AFTER EACH SESSION: call mempalace_diary_write to record what happened, what you learned, what matters.\n5. WHEN FACTS CHANGE: call mempalace_kg_invalidate on the old fact, mempalace_kg_add for the new one.\n\nThis protocol ensures the AI KNOWS before it speaks. Storage is not memory — but storage + this protocol = memory.";

const AAAK_SPEC: &str = "AAAK is a compressed memory dialect that MemPalace uses for efficient storage.\nIt is designed to be readable by both humans and LLMs without decoding.\n\nFORMAT:\n  ENTITIES: 3-letter uppercase codes. ALC=Alice, JOR=Jordan, RIL=Riley, MAX=Max, BEN=Ben.\n  EMOTIONS: *action markers* before/during text. *warm*=joy, *fierce*=determined, *raw*=vulnerable, *bloom*=tenderness.\n  STRUCTURE: Pipe-separated fields. FAM: family | PROJ: projects | ⚠: warnings/reminders.\n  DATES: ISO format (2026-03-31). COUNTS: Nx = N mentions (e.g., 570x).\n  IMPORTANCE: ★ to ★★★★★ (1-5 scale).\n  HALLS: hall_facts, hall_events, hall_discoveries, hall_preferences, hall_advice.\n  WINGS: wing_user, wing_agent, wing_team, wing_code, wing_myproject, wing_hardware, wing_ue5, wing_ai_research.\n  ROOMS: Hyphenated slugs representing named ideas (e.g., chromadb-setup, gpu-pricing).\n\nEXAMPLE:\n  FAM: ALC→♡JOR | 2D(kids): RIL(18,sports) MAX(11,chess+swimming) | BEN(contributor)\n\nRead AAAK naturally — expand codes mentally, treat *markers* as emotional context.\nWhen WRITING AAAK: use entity codes, mark emotions, keep structure tight.";

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
            "mempalace_get_aaak_spec",
            "Return the AAAK dialect specification.",
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
        "mempalace_get_aaak_spec" => Ok(json!({
            "aaak_spec": AAAK_SPEC,
        })),
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
    !matches!(tool_name, "mempalace_diary_write")
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
