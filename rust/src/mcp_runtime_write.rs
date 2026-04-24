use serde_json::{Value, json};

use crate::config::AppConfig;
use crate::error::{MempalaceError, Result};
use crate::mcp_runtime::{best_effort_wal_log, tool_error};
use crate::mcp_schema::{required_str, truncate_duplicate_content};
use crate::service::App;

pub async fn call_write_tool(
    name: &str,
    arguments: &Value,
    app: &App,
    config: &AppConfig,
) -> Result<Value> {
    match name {
        "mempalace_add_drawer" => {
            let wing = required_str(arguments, "wing", "mempalace_add_drawer");
            let Ok(wing) = wing else {
                return Ok(tool_error(
                    "Add drawer error",
                    &MempalaceError::Mcp("mempalace_add_drawer requires wing".to_string()),
                    "Provide wing, room, and content, then rerun mempalace_add_drawer.",
                ));
            };
            let room = required_str(arguments, "room", "mempalace_add_drawer");
            let Ok(room) = room else {
                return Ok(tool_error(
                    "Add drawer error",
                    &MempalaceError::Mcp("mempalace_add_drawer requires room".to_string()),
                    "Provide wing, room, and content, then rerun mempalace_add_drawer.",
                ));
            };
            let content = required_str(arguments, "content", "mempalace_add_drawer");
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
            let drawer_id = required_str(arguments, "drawer_id", "mempalace_delete_drawer");
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
                .unwrap_or(true);
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
        "mempalace_kg_add" => {
            let subject = required_str(arguments, "subject", "mempalace_kg_add");
            let Ok(subject) = subject else {
                return Ok(tool_error(
                    "KG add error",
                    &MempalaceError::Mcp("mempalace_kg_add requires subject".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_add.",
                ));
            };
            let predicate = required_str(arguments, "predicate", "mempalace_kg_add");
            let Ok(predicate) = predicate else {
                return Ok(tool_error(
                    "KG add error",
                    &MempalaceError::Mcp("mempalace_kg_add requires predicate".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_add.",
                ));
            };
            let object = required_str(arguments, "object", "mempalace_kg_add");
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
            let subject = required_str(arguments, "subject", "mempalace_kg_invalidate");
            let Ok(subject) = subject else {
                return Ok(tool_error(
                    "KG invalidate error",
                    &MempalaceError::Mcp("mempalace_kg_invalidate requires subject".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_invalidate.",
                ));
            };
            let predicate = required_str(arguments, "predicate", "mempalace_kg_invalidate");
            let Ok(predicate) = predicate else {
                return Ok(tool_error(
                    "KG invalidate error",
                    &MempalaceError::Mcp("mempalace_kg_invalidate requires predicate".to_string()),
                    "Provide subject, predicate, and object, then rerun mempalace_kg_invalidate.",
                ));
            };
            let object = required_str(arguments, "object", "mempalace_kg_invalidate");
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
        _ => Ok(json!({
            "error": {
                "code": -32601,
                "message": format!("Unknown write tool: {name}")
            }
        })),
    }
}
