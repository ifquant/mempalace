use serde_json::{Value, json};

use crate::config::AppConfig;
use crate::error::{MempalaceError, Result};
use crate::mcp_runtime::call_tool;
use crate::mcp_schema::{coerce_argument_types, negotiate_protocol, tools};

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
