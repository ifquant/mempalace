use serde_json::{Value, json};

use crate::config::AppConfig;
use crate::error::{MempalaceError, Result};
use crate::mcp_runtime::{best_effort_wal_log, tool_error};
use crate::mcp_schema::required_str;
use crate::service::App;

pub async fn call_registry_tool(
    name: &str,
    arguments: &Value,
    app: &App,
    config: &AppConfig,
) -> Result<Value> {
    match name {
        "mempalace_registry_summary" => {
            let project_dir = required_str(arguments, "project_dir", "mempalace_registry_summary");
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
            let project_dir = required_str(arguments, "project_dir", "mempalace_registry_lookup");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry lookup error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_lookup requires project_dir".to_string(),
                    ),
                    "Provide project_dir and word, then rerun mempalace_registry_lookup.",
                ));
            };
            let word = required_str(arguments, "word", "mempalace_registry_lookup");
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
            let project_dir = required_str(arguments, "project_dir", "mempalace_registry_query");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry query error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_query requires project_dir".to_string(),
                    ),
                    "Provide project_dir and query, then rerun mempalace_registry_query.",
                ));
            };
            let query = required_str(arguments, "query", "mempalace_registry_query");
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
            let project_dir = required_str(arguments, "project_dir", "mempalace_registry_learn");
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
                required_str(arguments, "project_dir", "mempalace_registry_add_person");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry add person error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_add_person requires project_dir".to_string(),
                    ),
                    "Provide project_dir and name, then rerun mempalace_registry_add_person.",
                ));
            };
            let name = required_str(arguments, "name", "mempalace_registry_add_person");
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
                required_str(arguments, "project_dir", "mempalace_registry_add_project");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry add project error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_add_project requires project_dir".to_string(),
                    ),
                    "Provide project_dir and name, then rerun mempalace_registry_add_project.",
                ));
            };
            let name = required_str(arguments, "name", "mempalace_registry_add_project");
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
                required_str(arguments, "project_dir", "mempalace_registry_add_alias");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry add alias error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_add_alias requires project_dir".to_string(),
                    ),
                    "Provide project_dir, canonical, and alias, then rerun mempalace_registry_add_alias.",
                ));
            };
            let canonical = required_str(arguments, "canonical", "mempalace_registry_add_alias");
            let Ok(canonical) = canonical else {
                return Ok(tool_error(
                    "Registry add alias error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_add_alias requires canonical".to_string(),
                    ),
                    "Provide project_dir, canonical, and alias, then rerun mempalace_registry_add_alias.",
                ));
            };
            let alias = required_str(arguments, "alias", "mempalace_registry_add_alias");
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
            let project_dir = required_str(arguments, "project_dir", "mempalace_registry_research");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry research error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_research requires project_dir".to_string(),
                    ),
                    "Provide project_dir and word, then rerun mempalace_registry_research.",
                ));
            };
            let word = required_str(arguments, "word", "mempalace_registry_research");
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
            let project_dir = required_str(arguments, "project_dir", "mempalace_registry_confirm");
            let Ok(project_dir) = project_dir else {
                return Ok(tool_error(
                    "Registry confirm error",
                    &MempalaceError::Mcp(
                        "mempalace_registry_confirm requires project_dir".to_string(),
                    ),
                    "Provide project_dir and word, then rerun mempalace_registry_confirm.",
                ));
            };
            let word = required_str(arguments, "word", "mempalace_registry_confirm");
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
        _ => Ok(json!({
            "error": {
                "code": -32601,
                "message": format!("Unknown registry tool: {name}")
            }
        })),
    }
}
