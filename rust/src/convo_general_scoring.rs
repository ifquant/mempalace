use std::collections::HashSet;

const DECISION_MARKERS: &[&str] = &[
    "let's use",
    "let’s use",
    "go with",
    "we should",
    "we decided",
    "we chose",
    "went with",
    "picked",
    "settled on",
    "instead of",
    "rather than",
    "because",
    "trade-off",
    "pros and cons",
    "architecture",
    "approach",
    "strategy",
    "pattern",
    "stack",
    "framework",
    "infrastructure",
    "configure",
    "default",
];

const PREFERENCE_MARKERS: &[&str] = &[
    "i prefer",
    "always use",
    "never use",
    "don't use",
    "dont use",
    "i like",
    "i hate",
    "please always",
    "please never",
    "my rule",
    "my preference",
    "my style",
    "my convention",
    "we always",
    "we never",
    "functional style",
    "snake_case",
    "camelcase",
    "camel_case",
    "tabs",
    "spaces",
];

const MILESTONE_MARKERS: &[&str] = &[
    "it works",
    "it worked",
    "got it working",
    "solved",
    "breakthrough",
    "figured it out",
    "nailed it",
    "finally",
    "first time",
    "first ever",
    "discovered",
    "realized",
    "found out",
    "turns out",
    "the key",
    "the trick",
    "built",
    "created",
    "implemented",
    "shipped",
    "launched",
    "deployed",
    "released",
    "prototype",
    "proof of concept",
    "demo",
];

const PROBLEM_MARKERS: &[&str] = &[
    "bug",
    "error",
    "crash",
    "fail",
    "failed",
    "broken",
    "issue",
    "problem",
    "doesn't work",
    "doesnt work",
    "not working",
    "won't work",
    "wont work",
    "root cause",
    "workaround",
    "the fix",
    "solution",
    "resolved",
    "patched",
    "stuck",
    "blocked",
];

const EMOTION_MARKERS: &[&str] = &[
    "love",
    "scared",
    "afraid",
    "proud",
    "hurt",
    "happy",
    "sad",
    "cry",
    "crying",
    "miss",
    "sorry",
    "grateful",
    "angry",
    "worried",
    "lonely",
    "beautiful",
    "amazing",
    "wonderful",
    "i feel",
    "i'm scared",
    "im scared",
    "i love you",
    "i'm sorry",
    "im sorry",
    "i can't",
    "i cant",
    "i wish",
    "i need",
    "never told anyone",
    "nobody knows",
];

const POSITIVE_WORDS: &[&str] = &[
    "pride",
    "proud",
    "joy",
    "happy",
    "love",
    "loving",
    "beautiful",
    "amazing",
    "wonderful",
    "incredible",
    "fantastic",
    "brilliant",
    "perfect",
    "excited",
    "thrilled",
    "grateful",
    "warm",
    "breakthrough",
    "success",
    "works",
    "working",
    "solved",
    "fixed",
    "nailed",
    "heart",
    "hug",
    "precious",
    "adore",
];

const NEGATIVE_WORDS: &[&str] = &[
    "bug",
    "error",
    "crash",
    "crashing",
    "crashed",
    "fail",
    "failed",
    "failing",
    "failure",
    "broken",
    "broke",
    "breaking",
    "issue",
    "problem",
    "wrong",
    "stuck",
    "blocked",
    "unable",
    "impossible",
    "missing",
    "terrible",
    "awful",
    "panic",
    "disaster",
    "mess",
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

pub fn score_segment(text: &str) -> Option<(String, f64)> {
    let mut scores = vec![
        ("decision", score_markers(text, DECISION_MARKERS)),
        ("preference", score_markers(text, PREFERENCE_MARKERS)),
        ("milestone", score_markers(text, MILESTONE_MARKERS)),
        ("problem", score_markers(text, PROBLEM_MARKERS)),
        ("emotional", score_markers(text, EMOTION_MARKERS)),
    ];
    scores.retain(|(_, score)| *score > 0.0);
    if scores.is_empty() {
        return None;
    }

    scores.sort_by(|left, right| right.1.total_cmp(&left.1));
    let (mut best_type, best_score) = scores[0];
    best_type = disambiguate(best_type, text, &scores);
    Some((best_type.to_string(), best_score))
}

pub fn confidence(score: f64, segment_len: usize) -> f64 {
    let length_bonus = if segment_len > 500 {
        2.0
    } else if segment_len > 200 {
        1.0
    } else {
        0.0
    };
    (score + length_bonus).min(3.0) / 3.0
}

fn score_markers(text: &str, markers: &[&str]) -> f64 {
    let lower = text.to_ascii_lowercase();
    markers
        .iter()
        .map(|marker| lower.matches(marker).count() as f64)
        .sum()
}

fn disambiguate<'a>(memory_type: &'a str, text: &str, scores: &[(&str, f64)]) -> &'a str {
    let sentiment = sentiment(text);
    let emotional_score = score_lookup(scores, "emotional");
    let milestone_score = score_lookup(scores, "milestone");

    if memory_type == "problem" && has_resolution(text) {
        if emotional_score > 0.0 && sentiment == Sentiment::Positive {
            return "emotional";
        }
        return "milestone";
    }

    if memory_type == "problem" && sentiment == Sentiment::Positive {
        if milestone_score > 0.0 {
            return "milestone";
        }
        if emotional_score > 0.0 {
            return "emotional";
        }
    }

    memory_type
}

fn score_lookup(scores: &[(&str, f64)], key: &str) -> f64 {
    scores
        .iter()
        .find_map(|(name, score)| (*name == key).then_some(*score))
        .unwrap_or(0.0)
}

fn sentiment(text: &str) -> Sentiment {
    let words = text
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let pos = POSITIVE_WORDS
        .iter()
        .filter(|word| words.contains(**word))
        .count();
    let neg = NEGATIVE_WORDS
        .iter()
        .filter(|word| words.contains(**word))
        .count();
    match pos.cmp(&neg) {
        std::cmp::Ordering::Greater => Sentiment::Positive,
        std::cmp::Ordering::Less => Sentiment::Negative,
        std::cmp::Ordering::Equal => Sentiment::Neutral,
    }
}

fn has_resolution(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "fixed",
        "solved",
        "resolved",
        "patched",
        "got it working",
        "it works",
        "nailed it",
        "figured it out",
        "the fix",
        "the answer",
        "the solution",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}
