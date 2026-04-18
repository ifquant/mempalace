use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use crate::registry::EntityRegistry;

const SYSTEM_DICT: &str = "/usr/share/dict/words";
const MIN_LENGTH: usize = 4;

const COMMON_TYPOS: &[(&str, &str)] = &[
    ("lsresdy", "already"),
    ("knoe", "know"),
    ("befor", "before"),
    ("teh", "the"),
    ("recieve", "receive"),
    ("wierd", "weird"),
    ("definately", "definitely"),
    ("seperate", "separate"),
    ("becuase", "because"),
    ("thier", "their"),
    ("enviroment", "environment"),
];

static HAS_DIGIT: OnceLock<Regex> = OnceLock::new();
static IS_CAMEL: OnceLock<Regex> = OnceLock::new();
static IS_ALLCAPS: OnceLock<Regex> = OnceLock::new();
static IS_TECHNICAL: OnceLock<Regex> = OnceLock::new();
static IS_URL: OnceLock<Regex> = OnceLock::new();
static IS_CODE_OR_EMOJI: OnceLock<Regex> = OnceLock::new();
static TOKEN_RE: OnceLock<Regex> = OnceLock::new();

static SYSTEM_WORDS: OnceLock<HashSet<String>> = OnceLock::new();
static SYSTEM_INDEX: OnceLock<BTreeMap<(char, usize), Vec<String>>> = OnceLock::new();

pub fn spellcheck_user_text(text: &str, known_names: &HashSet<String>) -> String {
    token_re()
        .replace_all(text, |caps: &regex::Captures<'_>| {
            let token = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
            fix_token(token, known_names)
        })
        .into_owned()
}

pub fn spellcheck_transcript(content: &str, known_names: &HashSet<String>) -> String {
    content
        .lines()
        .map(|line| spellcheck_transcript_line(line, known_names))
        .collect::<Vec<_>>()
        .join("\n")
}

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
    if stripped.is_empty() || should_skip(stripped, known_names) {
        return token.to_string();
    }

    if let Some(corrected) = common_typo_map().get(&stripped.to_ascii_lowercase()) {
        return format!("{corrected}{punct}");
    }

    if system_words().contains(&stripped.to_ascii_lowercase()) {
        return token.to_string();
    }

    if let Some(candidate) = best_dictionary_candidate(stripped) {
        return format!("{candidate}{punct}");
    }

    token.to_string()
}

fn should_skip(token: &str, known_names: &HashSet<String>) -> bool {
    if token.len() < MIN_LENGTH {
        return true;
    }
    if has_digit().is_match(token)
        || is_camel().is_match(token)
        || is_allcaps().is_match(token)
        || is_technical().is_match(token)
        || is_url().is_match(token)
        || is_code_or_emoji().is_match(token)
        || known_names.contains(&token.to_ascii_lowercase())
    {
        return true;
    }
    token.chars().next().is_some_and(|ch| ch.is_uppercase())
}

fn best_dictionary_candidate(token: &str) -> Option<String> {
    let lower = token.to_ascii_lowercase();
    let first = lower.chars().next()?;
    let len = lower.len();
    let max_edits = if len <= 7 { 2 } else { 3 };

    let mut best: Option<(usize, usize, String)> = None;
    for candidate_len in len.saturating_sub(2)..=(len + 2) {
        let Some(words) = system_index().get(&(first, candidate_len)) else {
            continue;
        };
        for candidate in words {
            let distance = edit_distance(&lower, candidate);
            if distance > max_edits {
                continue;
            }
            let rank = (distance, candidate_len.abs_diff(len), candidate.clone());
            if best.as_ref().is_none_or(|current| rank < *current) {
                best = Some(rank);
            }
        }
    }

    best.map(|(_, _, candidate)| candidate)
}

fn edit_distance(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev = (0..=b.len()).collect::<Vec<_>>();
    for (i, ca) in a.chars().enumerate() {
        let mut curr = vec![i + 1];
        for (j, cb) in b.chars().enumerate() {
            curr.push(
                (prev[j + 1] + 1)
                    .min(curr[j] + 1)
                    .min(prev[j] + usize::from(ca != cb)),
            );
        }
        prev = curr;
    }
    *prev.last().unwrap_or(&usize::MAX)
}

fn common_typo_map() -> &'static BTreeMap<String, String> {
    static MAP: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    MAP.get_or_init(|| {
        COMMON_TYPOS
            .iter()
            .map(|(wrong, right)| ((*wrong).to_string(), (*right).to_string()))
            .collect()
    })
}

fn system_words() -> &'static HashSet<String> {
    SYSTEM_WORDS.get_or_init(|| load_system_words().into_iter().collect())
}

fn system_index() -> &'static BTreeMap<(char, usize), Vec<String>> {
    SYSTEM_INDEX.get_or_init(|| {
        let mut index: BTreeMap<(char, usize), Vec<String>> = BTreeMap::new();
        for word in load_system_words() {
            let Some(first) = word.chars().next() else {
                continue;
            };
            index.entry((first, word.len())).or_default().push(word);
        }
        index
    })
}

fn load_system_words() -> Vec<String> {
    fs::read_to_string(SYSTEM_DICT)
        .ok()
        .map(|content| {
            content
                .lines()
                .map(|line| line.trim().to_ascii_lowercase())
                .filter(|line| !line.is_empty() && line.chars().all(|ch| ch.is_ascii_alphabetic()))
                .collect()
        })
        .unwrap_or_default()
}

fn has_digit() -> &'static Regex {
    HAS_DIGIT.get_or_init(|| Regex::new(r"\d").expect("digit regex"))
}

fn is_camel() -> &'static Regex {
    IS_CAMEL.get_or_init(|| Regex::new(r"[A-Z][a-z]+[A-Z]").expect("camel regex"))
}

fn is_allcaps() -> &'static Regex {
    IS_ALLCAPS
        .get_or_init(|| Regex::new(r"^[A-Z_@#$%^&*()+=\[\]{}|<>?.:/\\]+$").expect("allcaps regex"))
}

fn is_technical() -> &'static Regex {
    IS_TECHNICAL.get_or_init(|| Regex::new(r"[-_]").expect("technical regex"))
}

fn is_url() -> &'static Regex {
    IS_URL
        .get_or_init(|| Regex::new(r"https?://|www\.|/Users/|~/|\.[a-z]{2,4}$").expect("url regex"))
}

fn is_code_or_emoji() -> &'static Regex {
    IS_CODE_OR_EMOJI.get_or_init(|| Regex::new(r"[`*_#{}\[\]\\]").expect("code regex"))
}

fn token_re() -> &'static Regex {
    TOKEN_RE.get_or_init(|| Regex::new(r"(\S+)").expect("token regex"))
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
