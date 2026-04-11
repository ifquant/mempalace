use serde_json::{Value, json};

use crate::config::AppConfig;
use crate::error::{MempalaceError, Result};
use crate::service::App;

pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 2] = ["2025-11-25", "2025-03-26"];

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
            let arguments = request["params"]
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let result = call_tool(name, arguments, config).await?;
            Ok(Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string(&result)?,
                        }
                    ]
                }
            })))
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
        tool("mempalace_status", "Show palace overview and room counts"),
        tool("mempalace_list_wings", "List all wings with drawer counts"),
        tool(
            "mempalace_list_rooms",
            "List rooms, optionally filtered by wing",
        ),
        tool("mempalace_get_taxonomy", "Show full wing to room taxonomy"),
        tool(
            "mempalace_search",
            "Semantic search, optionally filtered by wing and room",
        ),
    ]
}

fn tool(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": {},
        }
    })
}

async fn call_tool(name: &str, arguments: Value, config: &AppConfig) -> Result<Value> {
    let app = App::new(config.clone());

    match name {
        "mempalace_status" => Ok(serde_json::to_value(app.status().await?)?),
        "mempalace_list_wings" => Ok(json!({ "wings": app.list_wings().await? })),
        "mempalace_list_rooms" => {
            let wing = arguments.get("wing").and_then(Value::as_str);
            Ok(serde_json::to_value(app.list_rooms(wing).await?)?)
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
            Ok(serde_json::to_value(
                app.search(query, wing, room, limit).await?,
            )?)
        }
        _ => Ok(json!({
            "error": {
                "code": -32601,
                "message": format!("Unknown tool: {name}")
            }
        })),
    }
}
