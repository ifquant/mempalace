use std::collections::HashSet;

use crate::spellcheck::{spellcheck_transcript, spellcheck_user_text};

pub(crate) fn count_quote_lines(text: &str) -> usize {
    text.lines()
        .filter(|line| line.trim_start().starts_with("> "))
        .count()
}

pub(crate) fn messages_to_transcript(
    messages: &[(&str, String)],
    known_names: &HashSet<String>,
) -> String {
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

pub(crate) fn normalize_quote_transcript(raw: &str, known_names: &HashSet<String>) -> String {
    spellcheck_transcript(raw, known_names)
}
