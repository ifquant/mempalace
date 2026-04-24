use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::sync::OnceLock;

const SYSTEM_DICT: &str = "/usr/share/dict/words";

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

static SYSTEM_WORDS: OnceLock<HashSet<String>> = OnceLock::new();
static SYSTEM_INDEX: OnceLock<BTreeMap<(char, usize), Vec<String>>> = OnceLock::new();

pub(crate) fn best_dictionary_candidate(token: &str) -> Option<String> {
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

pub(crate) fn common_typo_map() -> &'static BTreeMap<String, String> {
    static MAP: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    MAP.get_or_init(|| {
        COMMON_TYPOS
            .iter()
            .map(|(wrong, right)| ((*wrong).to_string(), (*right).to_string()))
            .collect()
    })
}

pub(crate) fn system_words() -> &'static HashSet<String> {
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
