//! Transcript spellchecking helpers.
//!
//! Spellchecking is intentionally narrow: only user-authored transcript turns
//! are corrected, and known project/entity names are treated as protected.

use std::collections::HashSet;
use std::path::Path;

use crate::registry::EntityRegistry;

#[path = "spellcheck_dict.rs"]
mod dict;
#[path = "spellcheck_rules.rs"]
mod rules;

/// Spellchecks free-form user text while preserving protected names.
pub fn spellcheck_user_text(text: &str, known_names: &HashSet<String>) -> String {
    rules::token_re()
        .replace_all(text, |caps: &regex::Captures<'_>| {
            let token = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
            fix_token(token, known_names)
        })
        .into_owned()
}

/// Spellchecks only quoted user turns inside an already formatted transcript.
pub fn spellcheck_transcript(content: &str, known_names: &HashSet<String>) -> String {
    content
        .lines()
        .map(|line| spellcheck_transcript_line(line, known_names))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Loads protected names by walking up from a file toward nearby registries.
pub fn known_names_for_path(path: &Path) -> HashSet<String> {
    let mut current = if path.is_dir() {
        Some(path)
    } else {
        path.parent()
    };
    let mut hops = 0usize;

    while let Some(dir) = current {
        let registry_path = dir.join("entity_registry.json");
        if registry_path.exists()
            && let Ok(registry) = EntityRegistry::load(&registry_path)
        {
            let mut names = HashSet::new();
            for key in registry.people.keys() {
                names.insert(key.to_ascii_lowercase());
            }
            for project in &registry.projects {
                names.insert(project.to_ascii_lowercase());
            }
            return names;
        }
        hops += 1;
        if hops >= 6 {
            break;
        }
        current = dir.parent();
    }

    HashSet::new()
}

fn spellcheck_transcript_line(line: &str, known_names: &HashSet<String>) -> String {
    let stripped = line.trim_start();
    if !stripped.starts_with('>') {
        // Assistant/output lines are pass-through so we do not mutate quoted
        // transcripts beyond the user-authored surface.
        return line.to_string();
    }

    let prefix_len = line.len() - stripped.len();
    let payload = stripped
        .strip_prefix("> ")
        .or_else(|| stripped.strip_prefix('>'))
        .unwrap_or_default();
    if payload.trim().is_empty() {
        return line.to_string();
    }

    let corrected = spellcheck_user_text(payload, known_names);
    format!("{}> {}", " ".repeat(prefix_len), corrected)
}

fn fix_token(token: &str, known_names: &HashSet<String>) -> String {
    let stripped = token.trim_end_matches(|ch: char| ".,!?;:'\")".contains(ch));
    let punct = &token[stripped.len()..];
    if stripped.is_empty() || rules::should_skip(stripped, known_names) {
        return token.to_string();
    }

    if let Some(corrected) = dict::common_typo_map().get(&stripped.to_ascii_lowercase()) {
        return format!("{corrected}{punct}");
    }

    if dict::system_words().contains(&stripped.to_ascii_lowercase()) {
        return token.to_string();
    }

    if let Some(candidate) = dict::best_dictionary_candidate(stripped) {
        return format!("{candidate}{punct}");
    }

    token.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellcheck_fixes_common_typos_but_preserves_names_and_technical_tokens() {
        let known_names = HashSet::from([
            "riley".to_string(),
            "mempalace".to_string(),
            "chromadb".to_string(),
        ]);

        let corrected = spellcheck_user_text(
            "Riley knoe the deploy befor lunch in MemPalace and ChromaDB.",
            &known_names,
        );

        assert!(corrected.contains("Riley know the deploy before lunch"));
        assert!(corrected.contains("MemPalace"));
        assert!(corrected.contains("ChromaDB"));
    }

    #[test]
    fn spellcheck_transcript_only_touches_user_turns() {
        let known_names = HashSet::from(["riley".to_string()]);
        let corrected = spellcheck_transcript(
            "> Riley knoe the answer befor noon\nAssistant knoe should stay as-is",
            &known_names,
        );

        assert!(corrected.contains("> Riley know the answer before noon"));
        assert!(corrected.contains("Assistant knoe should stay as-is"));
    }
}
