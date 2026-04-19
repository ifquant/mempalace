use std::collections::HashSet;

use serde_json::Value;

use crate::normalize_transcript::messages_to_transcript;

pub(crate) fn try_flat_messages_json(
    data: &Value,
    known_names: &HashSet<String>,
) -> Option<String> {
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

pub(crate) fn try_claude_ai_json(data: &Value, known_names: &HashSet<String>) -> Option<String> {
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

pub(crate) fn try_chatgpt_json(data: &Value, known_names: &HashSet<String>) -> Option<String> {
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

pub(crate) fn try_slack_json(data: &Value, known_names: &HashSet<String>) -> Option<String> {
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

pub(crate) fn extract_content(value: &Value) -> String {
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
