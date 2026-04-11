use std::path::Path;

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
    if !palace_exists(config) {
        return Ok(no_palace());
    }

    let app = App::new(config.clone())?;

    match name {
        "mempalace_status" => {
            let status = app.status().await?;
            Ok(json!({
                "total_drawers": status.total_drawers,
                "wings": status.wings,
                "rooms": status.rooms,
                "palace_path": status.palace_path,
                "protocol": PALACE_PROTOCOL,
                "aaak_dialect": AAAK_SPEC,
            }))
        }
        "mempalace_list_wings" => Ok(json!({ "wings": app.list_wings().await? })),
        "mempalace_list_rooms" => {
            let wing = arguments.get("wing").and_then(Value::as_str);
            let rooms = app.list_rooms(wing).await?;
            Ok(json!({
                "wing": rooms.wing,
                "rooms": rooms.rooms,
            }))
        }
        "mempalace_get_taxonomy" => Ok(serde_json::to_value(app.taxonomy().await?)?),
        "mempalace_search" => {
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    MempalaceError::Mcp("mempalace_search requires query".to_string())
                })?;
            let wing = arguments.get("wing").and_then(Value::as_str);
            let room = arguments.get("room").and_then(Value::as_str);
            let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize;
            let results = app.search(query, wing, room, limit).await?;
            Ok(json!({
                "query": query,
                "filters": {"wing": wing, "room": room},
                "results": results.results.into_iter().map(|hit| {
                    let source_file = Path::new(&hit.source_path)
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| hit.source_path.clone());
                    let similarity = hit.score.map(|score| (1.0 - score).clamp(0.0, 1.0));
                    json!({
                        "text": hit.text,
                        "wing": hit.wing,
                        "room": hit.room,
                        "source_file": source_file,
                        "similarity": similarity.map(|value| (value * 1000.0).round() / 1000.0),
                    })
                }).collect::<Vec<_>>()
            }))
        }
        _ => Ok(json!({
            "error": {
                "code": -32601,
                "message": format!("Unknown tool: {name}")
            }
        })),
    }
}

fn palace_exists(config: &AppConfig) -> bool {
    config.sqlite_path().exists() || config.lance_path().exists()
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

    if tool_name == "mempalace_search"
        && let Some(value) = args.get("limit").cloned()
    {
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
