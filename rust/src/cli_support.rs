use std::path::Path;

use anyhow::Result;
use mempalace_rs::config::AppConfig;
use serde_json::json;

pub fn apply_cli_overrides(config: &mut AppConfig, hf_endpoint: Option<&str>) {
    if let Some(endpoint) = hf_endpoint {
        config.embedding.hf_endpoint = Some(endpoint.to_string());
    }
}

pub fn palace_exists(config: &AppConfig) -> bool {
    config.sqlite_path().exists() || config.lance_path().exists()
}

pub fn print_no_palace(config: &AppConfig) -> Result<()> {
    let payload = json!({
        "error": "No palace found",
        "hint": "Run: mempalace init <dir> && mempalace mine <dir>",
        "palace_path": config.palace_path.display().to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

pub fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '-' | '_' | ':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub fn format_mcp_setup(palace_path: &Path) -> String {
    let base_server_cmd = "mempalace-rs mcp --serve";
    let current_server_cmd = format!(
        "{base_server_cmd} --palace {}",
        shell_quote(&palace_path.display().to_string())
    );

    format!(
        "MemPalace MCP quick setup:\n  claude mcp add mempalace -- {current}\n\nRun the server directly:\n  {current}\n\nOptional custom palace:\n  claude mcp add mempalace -- {base} --palace /path/to/palace\n  {base} --palace /path/to/palace\n",
        current = current_server_cmd,
        base = base_server_cmd
    )
}
