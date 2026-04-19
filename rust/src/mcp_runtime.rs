use serde_json::{Value, json};

use crate::audit::WriteAheadLog;
use crate::config::AppConfig;
use crate::error::Result;
use crate::mcp_runtime_project::call_project_tool;
use crate::mcp_runtime_read::call_read_tool;
use crate::mcp_runtime_registry::call_registry_tool;
use crate::mcp_runtime_write::call_write_tool;
use crate::mcp_schema::{no_palace, requires_existing_palace};
use crate::service::App;

pub async fn call_tool(name: &str, arguments: Value, config: &AppConfig) -> Result<Value> {
    if requires_existing_palace(name) && !palace_exists(config) {
        return Ok(no_palace());
    }

    let app = App::new(config.clone())?;

    match name {
        "mempalace_status"
        | "mempalace_list_wings"
        | "mempalace_list_rooms"
        | "mempalace_get_taxonomy"
        | "mempalace_search"
        | "mempalace_check_duplicate"
        | "mempalace_get_aaak_spec"
        | "mempalace_wake_up"
        | "mempalace_recall"
        | "mempalace_layers_status"
        | "mempalace_kg_query"
        | "mempalace_kg_timeline"
        | "mempalace_kg_stats"
        | "mempalace_diary_read"
        | "mempalace_traverse"
        | "mempalace_find_tunnels"
        | "mempalace_graph_stats" => call_read_tool(name, &arguments, &app).await,
        "mempalace_add_drawer"
        | "mempalace_delete_drawer"
        | "mempalace_repair"
        | "mempalace_repair_scan"
        | "mempalace_repair_prune"
        | "mempalace_repair_rebuild"
        | "mempalace_compress"
        | "mempalace_dedup"
        | "mempalace_kg_add"
        | "mempalace_kg_invalidate"
        | "mempalace_diary_write" => call_write_tool(name, &arguments, &app, config).await,
        "mempalace_onboarding"
        | "mempalace_normalize"
        | "mempalace_split"
        | "mempalace_instructions"
        | "mempalace_hook_run" => call_project_tool(name, &arguments, config).await,
        "mempalace_registry_summary"
        | "mempalace_registry_lookup"
        | "mempalace_registry_query"
        | "mempalace_registry_learn"
        | "mempalace_registry_add_person"
        | "mempalace_registry_add_project"
        | "mempalace_registry_add_alias"
        | "mempalace_registry_research"
        | "mempalace_registry_confirm" => call_registry_tool(name, &arguments, &app, config).await,
        _ => Ok(json!({
            "error": {
                "code": -32601,
                "message": format!("Unknown tool: {name}")
            }
        })),
    }
}

pub(crate) fn tool_error(prefix: &str, err: &dyn std::fmt::Display, hint: &str) -> Value {
    json!({
        "error": format!("{prefix}: {err}"),
        "hint": hint,
    })
}

pub(crate) fn palace_exists(config: &AppConfig) -> bool {
    config.sqlite_path().exists() || config.lance_path().exists()
}

pub(crate) fn best_effort_wal_log(config: &AppConfig, operation: &str, params: Value) {
    match WriteAheadLog::for_palace(&config.palace_path) {
        Ok(wal) => {
            if let Err(err) = wal.log(operation, params, None) {
                tracing::error!("WAL write failed: {err}");
            }
        }
        Err(err) => tracing::error!("WAL init failed: {err}"),
    }
}
