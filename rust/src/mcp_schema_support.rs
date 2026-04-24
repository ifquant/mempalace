use serde_json::{Value, json};

use crate::error::{MempalaceError, Result};

pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 4] =
    ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

pub const PALACE_PROTOCOL: &str = "IMPORTANT — MemPalace Memory Protocol:\n1. ON WAKE-UP: Call mempalace_status to load palace overview + AAAK spec.\n2. BEFORE RESPONDING about any person, project, or past event: call mempalace_kg_query or mempalace_search FIRST. Never guess — verify.\n3. IF UNSURE about a fact (name, gender, age, relationship): say \"let me check\" and query the palace. Wrong is worse than slow.\n4. AFTER EACH SESSION: call mempalace_diary_write to record what happened, what you learned, what matters.\n5. WHEN FACTS CHANGE: call mempalace_kg_invalidate on the old fact, mempalace_kg_add for the new one.\n\nThis protocol ensures the AI KNOWS before it speaks. Storage is not memory — but storage + this protocol = memory.";

pub fn negotiate_protocol(params: Option<&Value>) -> &'static str {
    let requested = params
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Value::as_str);

    match requested {
        Some(version) => SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .copied()
            .find(|supported| *supported == version)
            .unwrap_or(SUPPORTED_PROTOCOL_VERSIONS[0]),
        None => SUPPORTED_PROTOCOL_VERSIONS.last().unwrap(),
    }
}

pub fn requires_existing_palace(tool_name: &str) -> bool {
    !matches!(
        tool_name,
        "mempalace_diary_write"
            | "mempalace_add_drawer"
            | "mempalace_kg_add"
            | "mempalace_onboarding"
            | "mempalace_normalize"
            | "mempalace_split"
            | "mempalace_instructions"
            | "mempalace_hook_run"
            | "mempalace_registry_summary"
            | "mempalace_registry_lookup"
            | "mempalace_registry_query"
            | "mempalace_registry_learn"
            | "mempalace_registry_add_person"
            | "mempalace_registry_add_project"
            | "mempalace_registry_add_alias"
            | "mempalace_registry_research"
            | "mempalace_registry_confirm"
    )
}

pub fn no_palace() -> Value {
    json!({
        "error": "No palace found",
        "hint": "Run: mempalace init <dir> && mempalace mine <dir>",
    })
}

pub fn coerce_argument_types(tool_name: &str, arguments: &mut Value) {
    let Some(args) = arguments.as_object_mut() else {
        return;
    };

    match tool_name {
        "mempalace_search" | "mempalace_recall" => {
            coerce_u64(args, "limit");
        }
        "mempalace_check_duplicate" => {
            coerce_f64(args, "threshold");
        }
        "mempalace_dedup" => {
            coerce_f64(args, "threshold");
            coerce_bool(args, "dry_run");
            coerce_bool(args, "stats_only");
            coerce_u64(args, "min_count");
        }
        "mempalace_repair_prune" | "mempalace_compress" => {
            coerce_bool(args, "confirm");
            coerce_bool(args, "dry_run");
        }
        "mempalace_onboarding" => {
            coerce_bool(args, "scan");
            coerce_bool(args, "auto_accept_detected");
        }
        "mempalace_split" => {
            coerce_bool(args, "dry_run");
            coerce_u64(args, "min_sessions");
        }
        "mempalace_hook_run" => {
            coerce_bool(args, "stop_hook_active");
        }
        "mempalace_registry_research" => {
            coerce_bool(args, "auto_confirm");
        }
        "mempalace_traverse" => {
            coerce_u64(args, "max_hops");
        }
        "mempalace_diary_read" => {
            coerce_u64(args, "last_n");
        }
        "mempalace_kg_add"
        | "mempalace_kg_invalidate"
        | "mempalace_add_drawer"
        | "mempalace_delete_drawer" => {}
        _ => {}
    }
}

pub fn truncate_duplicate_content(text: &str) -> String {
    const PREVIEW_LIMIT: usize = 200;
    if text.chars().count() <= PREVIEW_LIMIT {
        text.to_string()
    } else {
        let preview = text.chars().take(PREVIEW_LIMIT).collect::<String>();
        format!("{preview}...")
    }
}

pub fn required_str<'a>(arguments: &'a Value, key: &str, tool_name: &str) -> Result<&'a str> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| MempalaceError::Mcp(format!("{tool_name} requires {key}")))
}

pub fn string_list_arg(arguments: &Value, key: &str) -> Vec<String> {
    match arguments.get(key) {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn coerce_bool(args: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = args.get(key).cloned() {
        let coerced = match value {
            Value::String(text) => text.parse::<bool>().ok().map(Value::from),
            Value::Bool(_) => Some(value),
            _ => None,
        };
        if let Some(flag) = coerced {
            args.insert(key.to_string(), flag);
        }
    }
}

fn coerce_u64(args: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = args.get(key).cloned() {
        let coerced = match value {
            Value::String(text) => text.parse::<u64>().ok().map(Value::from),
            Value::Number(_) => value.as_u64().map(Value::from),
            _ => None,
        };
        if let Some(number) = coerced {
            args.insert(key.to_string(), number);
        }
    }
}

fn coerce_f64(args: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = args.get(key).cloned() {
        let coerced = match value {
            Value::String(text) => text.parse::<f64>().ok().map(Value::from),
            Value::Number(_) => value.as_f64().map(Value::from),
            _ => None,
        };
        if let Some(number) = coerced {
            args.insert(key.to_string(), number);
        }
    }
}
