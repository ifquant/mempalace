use std::collections::HashSet;

use serde_json::Value;

use crate::normalize_transcript::messages_to_transcript;

use super::exports::extract_content;

pub(crate) fn try_claude_code_jsonl(
    content: &str,
    known_names: &HashSet<String>,
) -> Option<String> {
    let mut messages = Vec::new();
    for line in content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let entry: Value = serde_json::from_str(line).ok()?;
        let msg_type = entry
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let message = entry.get("message").unwrap_or(&Value::Null);
        match msg_type {
            "human" | "user" => {
                let text = extract_content(message.get("content").unwrap_or(&Value::Null));
                if !text.is_empty() {
                    messages.push(("user", text));
                }
            }
            "assistant" => {
                let text = extract_content(message.get("content").unwrap_or(&Value::Null));
                if !text.is_empty() {
                    messages.push(("assistant", text));
                }
            }
            _ => {}
        }
    }
    (messages.len() >= 2).then(|| messages_to_transcript(&messages, known_names))
}

pub(crate) fn try_codex_jsonl(content: &str, known_names: &HashSet<String>) -> Option<String> {
    let mut messages = Vec::new();
    let mut has_session_meta = false;
    for line in content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let entry: Value = serde_json::from_str(line).ok()?;
        let entry_type = entry
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if entry_type == "session_meta" {
            has_session_meta = true;
            continue;
        }
        if entry_type != "event_msg" {
            continue;
        }
        let payload = entry.get("payload")?;
        let payload_type = payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let text = payload
            .get("message")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if text.is_empty() {
            continue;
        }
        match payload_type {
            "user_message" => messages.push(("user", text.to_string())),
            "agent_message" => messages.push(("assistant", text.to_string())),
            _ => {}
        }
    }
    (messages.len() >= 2 && has_session_meta)
        .then(|| messages_to_transcript(&messages, known_names))
}
