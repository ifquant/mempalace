//! JSON and JSONL normalization facade.
//!
//! Format-specific parsing lives in split modules so reviewers can inspect
//! export handlers separately from line-oriented JSONL heuristics.

use std::collections::HashSet;

use serde_json::Value;

#[path = "normalize_json_exports.rs"]
mod exports;
#[path = "normalize_json_jsonl.rs"]
mod jsonl;

pub(crate) fn try_normalize_json(content: &str, known_names: &HashSet<String>) -> Option<String> {
    if let Some(transcript) = jsonl::try_claude_code_jsonl(content, known_names) {
        return Some(transcript);
    }
    if let Some(transcript) = jsonl::try_codex_jsonl(content, known_names) {
        return Some(transcript);
    }

    let data: Value = serde_json::from_str(content).ok()?;
    exports::try_flat_messages_json(&data, known_names)
        .or_else(|| exports::try_claude_ai_json(&data, known_names))
        .or_else(|| exports::try_chatgpt_json(&data, known_names))
        .or_else(|| exports::try_slack_json(&data, known_names))
}
