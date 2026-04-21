use std::collections::{BTreeMap, HashSet};

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
        let Some(node) = mapping.get(&current) else {
            break;
        };
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
                        .filter(|text| !text.is_empty())
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
    let mut seen_users = BTreeMap::<String, &'static str>::new();
    let mut last_role: Option<&'static str> = None;

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

        let role = if let Some(role) = seen_users.get(user_id) {
            *role
        } else if seen_users.is_empty() {
            seen_users.insert(user_id.to_string(), "user");
            "user"
        } else if last_role == Some("user") {
            seen_users.insert(user_id.to_string(), "assistant");
            "assistant"
        } else {
            seen_users.insert(user_id.to_string(), "user");
            "user"
        };
        last_role = Some(role);
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::json;

    use super::try_slack_json;

    #[test]
    fn slack_json_preserves_assigned_role_for_returning_third_user() {
        let data = json!([
            {"type":"message", "user":"A", "text":"first from A"},
            {"type":"message", "user":"B", "text":"first from B"},
            {"type":"message", "user":"A", "text":"second from A"},
            {"type":"message", "user":"C", "text":"first from C"},
            {"type":"message", "user":"C", "text":"second from C"}
        ]);

        let normalized = try_slack_json(&data, &HashSet::new()).unwrap();

        assert!(normalized.contains("> first from A"));
        assert!(normalized.contains("first from B"));
        assert!(normalized.contains("> second from A"));
        assert!(normalized.contains("first from C"));
        assert!(normalized.contains("second from C"));
        assert!(!normalized.contains("> second from C"));
    }

    #[test]
    fn chatgpt_json_keeps_messages_before_missing_child_like_python() {
        let data = json!({
            "mapping": {
                "root": {"parent": null, "message": null, "children": ["u1"]},
                "u1": {
                    "parent": "root",
                    "message": {"author": {"role": "user"}, "content": {"parts": ["How do we ship this?"]}},
                    "children": ["a1"]
                },
                "a1": {
                    "parent": "u1",
                    "message": {"author": {"role": "assistant"}, "content": {"parts": ["Run tests first."]}},
                    "children": ["missing-node"]
                }
            }
        });

        let normalized = super::try_chatgpt_json(&data, &HashSet::new()).unwrap();

        assert!(normalized.contains("> How do we ship this?"));
        assert!(normalized.contains("Run tests first."));
    }

    #[test]
    fn chatgpt_json_ignores_empty_parts_like_python() {
        let data = json!({
            "mapping": {
                "root": {"parent": null, "message": null, "children": ["u1"]},
                "u1": {
                    "parent": "root",
                    "message": {"author": {"role": "user"}, "content": {"parts": ["Ship", "", "today"]}},
                    "children": ["a1"]
                },
                "a1": {
                    "parent": "u1",
                    "message": {"author": {"role": "assistant"}, "content": {"parts": ["Run", "", "tests"]}},
                    "children": []
                }
            }
        });

        let normalized = super::try_chatgpt_json(&data, &HashSet::new()).unwrap();

        assert!(normalized.contains("> Ship today"));
        assert!(normalized.contains("Run tests"));
        assert!(!normalized.contains("Ship  today"));
        assert!(!normalized.contains("Run  tests"));
    }
}
