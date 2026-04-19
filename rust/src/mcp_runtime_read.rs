use serde_json::{Value, json};

use crate::dialect::AAAK_SPEC;
use crate::error::{MempalaceError, Result};
use crate::mcp_runtime::tool_error;
use crate::mcp_schema::{PALACE_PROTOCOL, truncate_duplicate_content};
use crate::service::App;

pub async fn call_read_tool(name: &str, arguments: &Value, app: &App) -> Result<Value> {
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
                "message": format!("Unknown read tool: {name}")
            }
        })),
    }
}
