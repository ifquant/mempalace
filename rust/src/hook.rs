use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::Local;
use serde_json::{Value, json};

use crate::config::AppConfig;
use crate::error::{MempalaceError, Result};

const SAVE_INTERVAL: usize = 15;

const STOP_BLOCK_REASON: &str = "AUTO-SAVE checkpoint. Save key topics, decisions, quotes, and code from this session to your memory system. Organize into appropriate categories. Use verbatim quotes where possible. Continue conversation after saving.";

const PRECOMPACT_BLOCK_REASON: &str = "COMPACTION IMMINENT. Save ALL topics, decisions, quotes, code, and important context from this session to your memory system. Be thorough — after compaction, detailed context will be lost. Organize into appropriate categories. Use verbatim quotes where possible. Save everything, then allow compaction to proceed.";

pub fn run_hook(hook_name: &str, harness: &str, config: &AppConfig) -> Result<Value> {
    let mut raw = String::new();
    let _ = std::io::stdin().read_to_string(&mut raw);
    let data = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
    run_hook_with_data(hook_name, harness, &data, config)
}

pub fn run_hook_with_data(
    hook_name: &str,
    harness: &str,
    data: &Value,
    config: &AppConfig,
) -> Result<Value> {
    let parsed = parse_harness_input(data, harness)?;

    match hook_name {
        "session-start" => hook_session_start(&parsed, config),
        "stop" => hook_stop(&parsed, config),
        "precompact" => hook_precompact(&parsed, config),
        other => Err(MempalaceError::InvalidArgument(format!(
            "Unknown hook: {other}"
        ))),
    }
}

#[derive(Clone, Debug)]
struct HookInput {
    session_id: String,
    stop_hook_active: bool,
    transcript_path: String,
}

fn parse_harness_input(data: &Value, harness: &str) -> Result<HookInput> {
    if !matches!(harness, "claude-code" | "codex") {
        return Err(MempalaceError::InvalidArgument(format!(
            "Unknown harness: {harness}"
        )));
    }

    Ok(HookInput {
        session_id: sanitize_session_id(
            data.get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
        ),
        stop_hook_active: data
            .get("stop_hook_active")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        transcript_path: data
            .get("transcript_path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

fn hook_session_start(input: &HookInput, config: &AppConfig) -> Result<Value> {
    log_message(
        config,
        &format!("SESSION START for session {}", input.session_id),
    );
    ensure_state_dir(config)?;
    Ok(json!({}))
}

fn hook_stop(input: &HookInput, config: &AppConfig) -> Result<Value> {
    if input.stop_hook_active {
        return Ok(json!({}));
    }

    let exchange_count = count_human_messages(&input.transcript_path);
    ensure_state_dir(config)?;
    let last_save_file = state_dir(config).join(format!("{}_last_save", input.session_id));
    let last_save = fs::read_to_string(&last_save_file)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let since_last = exchange_count.saturating_sub(last_save);
    log_message(
        config,
        &format!(
            "Session {}: {} exchanges, {} since last save",
            input.session_id, exchange_count, since_last
        ),
    );

    if since_last >= SAVE_INTERVAL && exchange_count > 0 {
        let _ = fs::write(&last_save_file, exchange_count.to_string());
        log_message(
            config,
            &format!("TRIGGERING SAVE at exchange {exchange_count}"),
        );
        maybe_auto_ingest(config);
        Ok(json!({
            "decision": "block",
            "reason": STOP_BLOCK_REASON,
        }))
    } else {
        Ok(json!({}))
    }
}

fn hook_precompact(input: &HookInput, config: &AppConfig) -> Result<Value> {
    log_message(
        config,
        &format!("PRE-COMPACT triggered for session {}", input.session_id),
    );
    maybe_auto_ingest_sync(config);
    Ok(json!({
        "decision": "block",
        "reason": PRECOMPACT_BLOCK_REASON,
    }))
}

fn sanitize_session_id(session_id: &str) -> String {
    let sanitized = session_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .collect::<String>();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

fn count_human_messages(transcript_path: &str) -> usize {
    let path = Path::new(transcript_path).expand_tilde();
    if !path.is_file() {
        return 0;
    }
    let Ok(contents) = fs::read_to_string(path) else {
        return 0;
    };

    let mut count = 0usize;
    for line in contents.lines() {
        let Ok(entry) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(message) = entry.get("message").and_then(Value::as_object)
            && message
                .get("role")
                .and_then(Value::as_str)
                .is_some_and(|role| role == "user")
        {
            let content = message.get("content");
            if contains_command_message(content) {
                continue;
            }
            count += 1;
            continue;
        }

        if entry.get("type").and_then(Value::as_str) == Some("event_msg")
            && let Some(payload) = entry.get("payload").and_then(Value::as_object)
            && payload.get("type").and_then(Value::as_str) == Some("user_message")
        {
            let message = payload.get("message").and_then(Value::as_str).unwrap_or("");
            if !message.contains("<command-message>") {
                count += 1;
            }
        }
    }

    count
}

fn contains_command_message(content: Option<&Value>) -> bool {
    match content {
        Some(Value::String(text)) => text.contains("<command-message>"),
        Some(Value::Array(blocks)) => blocks.iter().any(|block| {
            block
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|text| text.contains("<command-message>"))
        }),
        _ => false,
    }
}

fn maybe_auto_ingest(config: &AppConfig) {
    let Some(mempal_dir) = std::env::var("MEMPAL_DIR")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
    else {
        return;
    };

    let log_path = state_dir(config).join("hook.log");
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok();
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok();
    let current_exe = std::env::current_exe().ok();
    let Some(current_exe) = current_exe else {
        return;
    };
    let mut command = Command::new(current_exe);
    command
        .arg("--palace")
        .arg(&config.palace_path)
        .arg("mine")
        .arg(&mempal_dir);
    if let Some(file) = stdout {
        command.stdout(Stdio::from(file));
    }
    if let Some(file) = stderr {
        command.stderr(Stdio::from(file));
    }
    let _ = command.spawn();
}

fn maybe_auto_ingest_sync(config: &AppConfig) {
    let Some(mempal_dir) = std::env::var("MEMPAL_DIR")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
    else {
        return;
    };

    let log_path = state_dir(config).join("hook.log");
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok();
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok();
    let current_exe = std::env::current_exe().ok();
    let Some(current_exe) = current_exe else {
        return;
    };
    let mut command = Command::new(current_exe);
    command
        .arg("--palace")
        .arg(&config.palace_path)
        .arg("mine")
        .arg(&mempal_dir);
    if let Some(file) = stdout {
        command.stdout(Stdio::from(file));
    }
    if let Some(file) = stderr {
        command.stderr(Stdio::from(file));
    }
    let _ = command.status();
}

fn ensure_state_dir(config: &AppConfig) -> Result<()> {
    fs::create_dir_all(state_dir(config))?;
    Ok(())
}

fn state_dir(config: &AppConfig) -> PathBuf {
    config.palace_path.join("hook_state")
}

fn log_message(config: &AppConfig, message: &str) {
    if ensure_state_dir(config).is_err() {
        return;
    }
    let log_path = state_dir(config).join("hook.log");
    let timestamp = Local::now().format("%H:%M:%S");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(file, "[{timestamp}] {message}");
    }
}

trait ExpandTilde {
    fn expand_tilde(&self) -> PathBuf;
}

impl ExpandTilde for Path {
    fn expand_tilde(&self) -> PathBuf {
        let raw = self.to_string_lossy();
        if raw == "~" {
            return dirs::home_dir().unwrap_or_else(|| self.to_path_buf());
        }
        if let Some(stripped) = raw.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(stripped);
        }
        self.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_human_messages_skips_command_messages() {
        let dir = tempfile::tempdir().unwrap();
        let transcript = dir.path().join("transcript.jsonl");
        fs::write(
            &transcript,
            concat!(
                "{\"message\":{\"role\":\"user\",\"content\":\"hello\"}}\n",
                "{\"message\":{\"role\":\"user\",\"content\":\"<command-message>skip</command-message>\"}}\n",
                "{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\",\"message\":\"hi\"}}\n",
                "{\"type\":\"event_msg\",\"payload\":{\"type\":\"user_message\",\"message\":\"<command-message>skip</command-message>\"}}\n"
            ),
        )
        .unwrap();

        assert_eq!(count_human_messages(transcript.to_str().unwrap()), 2);
    }
}
