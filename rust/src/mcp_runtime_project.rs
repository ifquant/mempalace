use serde_json::{Value, json};

use crate::config::AppConfig;
use crate::error::{MempalaceError, Result};
use crate::hook;
use crate::instructions;
use crate::mcp_runtime::tool_error;
use crate::mcp_schema::{required_str, string_list_arg};
use crate::normalize::normalize_conversation_file;
use crate::onboarding::{OnboardingRequest, parse_alias_arg, parse_person_arg, run_onboarding};
use crate::split;

pub async fn call_project_tool(name: &str, arguments: &Value, config: &AppConfig) -> Result<Value> {
    match name {
        "mempalace_onboarding" => {
            let project_dir = required_str(arguments, "project_dir", "mempalace_onboarding");
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
                projects: string_list_arg(arguments, "projects"),
                aliases: std::collections::BTreeMap::new(),
                wings: string_list_arg(arguments, "wings"),
                scan: arguments.get("scan").and_then(Value::as_bool),
                auto_accept_detected: arguments
                    .get("auto_accept_detected")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            };

            for person in string_list_arg(arguments, "people") {
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
            for alias in string_list_arg(arguments, "aliases") {
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
            let file_path = required_str(arguments, "file_path", "mempalace_normalize");
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
            let source_dir = required_str(arguments, "source_dir", "mempalace_split");
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
            let name = required_str(arguments, "name", "mempalace_instructions");
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
            let hook_name = required_str(arguments, "hook", "mempalace_hook_run");
            let Ok(hook_name) = hook_name else {
                return Ok(tool_error(
                    "Hook run error",
                    &MempalaceError::Mcp("mempalace_hook_run requires hook".to_string()),
                    "Provide hook and harness, then rerun mempalace_hook_run.",
                ));
            };
            let harness = required_str(arguments, "harness", "mempalace_hook_run");
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
        _ => Ok(json!({
            "error": {
                "code": -32601,
                "message": format!("Unknown project tool: {name}")
            }
        })),
    }
}
