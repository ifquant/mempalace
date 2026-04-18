use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::Result;
use crate::spellcheck::{known_names_for_path, spellcheck_transcript, spellcheck_user_text};

pub fn normalize_conversation_file(path: &Path) -> Result<Option<String>> {
    let known_names = known_names_for_path(path);
    let raw = match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => return Ok(None),
        },
        Err(err) => return Err(err.into()),
    };
    normalize_conversation(path, &raw, &known_names)
}

pub fn normalize_conversation(
    path: &Path,
    raw: &str,
    known_names: &HashSet<String>,
) -> Result<Option<String>> {
    let content = raw.trim();
    if content.is_empty() {
        return Ok(None);
    }

    if count_quote_lines(content) >= 3 {
        return Ok(Some(spellcheck_transcript(raw, known_names)));
    }

    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    if matches!(ext.as_str(), "json" | "jsonl")
        || matches!(content.chars().next(), Some('{') | Some('['))
    {
        if let Some(normalized) = try_normalize_json(content, known_names) {
            return Ok(Some(normalized));
        }
        if matches!(ext.as_str(), "json" | "jsonl") {
            return Ok(None);
        }
    }

    Ok(Some(raw.to_string()))
}

fn count_quote_lines(text: &str) -> usize {
    text.lines()
        .filter(|line| line.trim_start().starts_with("> "))
        .count()
}

fn try_normalize_json(content: &str, known_names: &HashSet<String>) -> Option<String> {
    if let Some(transcript) = try_claude_code_jsonl(content, known_names) {
        return Some(transcript);
    }
    if let Some(transcript) = try_codex_jsonl(content, known_names) {
        return Some(transcript);
    }

    let data: Value = serde_json::from_str(content).ok()?;
    try_flat_messages_json(&data, known_names)
        .or_else(|| try_claude_ai_json(&data, known_names))
        .or_else(|| try_chatgpt_json(&data, known_names))
        .or_else(|| try_slack_json(&data, known_names))
}

fn try_claude_code_jsonl(content: &str, known_names: &HashSet<String>) -> Option<String> {
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

fn try_codex_jsonl(content: &str, known_names: &HashSet<String>) -> Option<String> {
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

fn try_flat_messages_json(data: &Value, known_names: &HashSet<String>) -> Option<String> {
    let items = data.as_array()?;
    let mut messages = Vec::new();
    for item in items {
        let role = item.get("role").and_then(Value::as_str).unwrap_or_default();
        let text = extract_content(item.get("content").unwrap_or(&Value::Null));
        if text.is_empty() {
            continue;
        }
        match role {
            "user" | "human" => messages.push(("user", text)),
            "assistant" | "ai" => messages.push(("assistant", text)),
            _ => {}
        }
    }
    (messages.len() >= 2).then(|| messages_to_transcript(&messages, known_names))
}

fn try_claude_ai_json(data: &Value, known_names: &HashSet<String>) -> Option<String> {
    let list = if let Some(messages) = data.get("messages").and_then(Value::as_array) {
        messages.clone()
    } else if let Some(messages) = data.get("chat_messages").and_then(Value::as_array) {
        messages.clone()
    } else {
        data.as_array()?.clone()
    };

    if list
        .first()
        .is_some_and(|item| item.get("chat_messages").is_some())
    {
        let mut all_messages = Vec::new();
        for convo in list {
            for item in convo
                .get("chat_messages")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let role = item.get("role").and_then(Value::as_str).unwrap_or_default();
                let text = extract_content(item.get("content").unwrap_or(&Value::Null));
                if text.is_empty() {
                    continue;
                }
                match role {
                    "user" | "human" => all_messages.push(("user", text)),
                    "assistant" | "ai" => all_messages.push(("assistant", text)),
                    _ => {}
                }
            }
        }
        return (all_messages.len() >= 2)
            .then(|| messages_to_transcript(&all_messages, known_names));
    }

    let mut messages = Vec::new();
    for item in list {
        let role = item.get("role").and_then(Value::as_str).unwrap_or_default();
        let text = extract_content(item.get("content").unwrap_or(&Value::Null));
        if text.is_empty() {
            continue;
        }
        match role {
            "user" | "human" => messages.push(("user", text)),
            "assistant" | "ai" => messages.push(("assistant", text)),
            _ => {}
        }
    }
    (messages.len() >= 2).then(|| messages_to_transcript(&messages, known_names))
}

fn try_chatgpt_json(data: &Value, known_names: &HashSet<String>) -> Option<String> {
    let mapping = data.get("mapping")?.as_object()?;
    let mut root_id = None;
    let mut fallback_root = None;
    for (node_id, node) in mapping {
        if node.get("parent").is_some_and(Value::is_null) {
            if node.get("message").is_some_and(Value::is_null) {
                root_id = Some(node_id.clone());
                break;
            }
            if fallback_root.is_none() {
                fallback_root = Some(node_id.clone());
            }
        }
    }

    let mut current = root_id.or(fallback_root)?;
    let mut messages = Vec::new();
    let mut visited = HashSet::new();
    while visited.insert(current.clone()) {
        let node = mapping.get(&current)?;
        if let Some(message) = node.get("message") {
            let role = message
                .get("author")
                .and_then(|author| author.get("role"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let text = message
                .get("content")
                .and_then(|content| content.get("parts"))
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string()
                })
                .unwrap_or_default();
            if !text.is_empty() {
                match role {
                    "user" => messages.push(("user", text)),
                    "assistant" => messages.push(("assistant", text)),
                    _ => {}
                }
            }
        }
        let next = node
            .get("children")
            .and_then(Value::as_array)
            .and_then(|children| children.first())
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        match next {
            Some(next) => current = next,
            None => break,
        }
    }

    (messages.len() >= 2).then(|| messages_to_transcript(&messages, known_names))
}

fn try_slack_json(data: &Value, known_names: &HashSet<String>) -> Option<String> {
    let items = data.as_array()?;
    let mut messages = Vec::new();
    let mut seen_users = Vec::<String>::new();
    let mut last_role = "assistant";

    for item in items {
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        let user_id = item
            .get("user")
            .or_else(|| item.get("username"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let text = item
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if user_id.is_empty() || text.is_empty() {
            continue;
        }

        let role = if let Some(index) = seen_users.iter().position(|user| user == user_id) {
            if index % 2 == 0 { "user" } else { "assistant" }
        } else {
            seen_users.push(user_id.to_string());
            if last_role == "user" {
                "assistant"
            } else {
                "user"
            }
        };
        last_role = role;
        messages.push((role, text.to_string()));
    }

    (messages.len() >= 2).then(|| messages_to_transcript(&messages, known_names))
}

fn extract_content(value: &Value) -> String {
    match value {
        Value::String(text) => text.trim().to_string(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(text) => Some(text.to_string()),
                Value::Object(map) if map.get("type").and_then(Value::as_str) == Some("text") => {
                    map.get("text")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string(),
        Value::Object(map) => map
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

fn messages_to_transcript(messages: &[(&str, String)], known_names: &HashSet<String>) -> String {
    let mut lines = Vec::new();
    let mut index = 0usize;
    while index < messages.len() {
        let (role, text) = &messages[index];
        if *role == "user" {
            lines.push(format!(
                "> {}",
                spellcheck_user_text(text.trim(), known_names)
            ));
            if let Some((next_role, next_text)) = messages.get(index + 1)
                && *next_role == "assistant"
            {
                lines.push(next_text.trim().to_string());
                index += 1;
            }
        } else {
            lines.push(text.trim().to_string());
        }
        lines.push(String::new());
        index += 1;
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;

    use super::normalize_conversation;

    #[test]
    fn normalize_json_transcript_spellchecks_user_turns() {
        let known_names = HashSet::from(["riley".to_string()]);
        let normalized = normalize_conversation(
            Path::new("demo.jsonl"),
            r#"{"type":"session_meta","payload":{"id":"demo"}}
{"type":"event_msg","payload":{"type":"user_message","message":"Riley knoe the deploy befor lunch"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"We fixed it."}}
"#,
            &known_names,
        )
        .unwrap()
        .unwrap();

        assert!(normalized.contains("> Riley know the deploy before lunch"));
        assert!(normalized.contains("We fixed it."));
    }

    #[test]
    fn normalize_chatgpt_export_to_transcript() {
        let known_names = HashSet::new();
        let normalized = normalize_conversation(
            Path::new("chatgpt.json"),
            r#"{
  "mapping": {
    "root": {"id":"root","parent":null,"message":null,"children":["u1"]},
    "u1": {"id":"u1","parent":"root","message":{"author":{"role":"user"},"content":{"parts":["How do we ship this?"]}},"children":["a1"]},
    "a1": {"id":"a1","parent":"u1","message":{"author":{"role":"assistant"},"content":{"parts":["Run tests first."]}},"children":[]}
  }
}"#,
            &known_names,
        )
        .unwrap()
        .unwrap();

        assert!(normalized.contains("> How do we ship this?"));
        assert!(normalized.contains("Run tests first."));
    }
}
