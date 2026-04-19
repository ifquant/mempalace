use std::collections::HashSet;

use regex::Regex;

use crate::convo::ConversationChunk;

const GENERAL_TYPES: &[&str] = &[
    "decision",
    "preference",
    "milestone",
    "problem",
    "emotional",
];

const LINE_GROUP_SIZE: usize = 25;

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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SpeakerRole {
    User,
    Assistant,
    Unknown,
}

pub fn general_rooms() -> Vec<String> {
    GENERAL_TYPES
        .iter()
        .map(|item| (*item).to_string())
        .collect()
}

pub fn extract_general_memories(text: &str, min_confidence: f64) -> Vec<ConversationChunk> {
    let segments = split_into_segments(text);
    let mut memories = Vec::new();

    for segment in segments {
        if segment.trim().len() < 20 {
            continue;
        }

        let prose = extract_prose(&segment);
        let mut scores = vec![
            ("decision", score_markers(&prose, DECISION_MARKERS)),
            ("preference", score_markers(&prose, PREFERENCE_MARKERS)),
            ("milestone", score_markers(&prose, MILESTONE_MARKERS)),
            ("problem", score_markers(&prose, PROBLEM_MARKERS)),
            ("emotional", score_markers(&prose, EMOTION_MARKERS)),
        ];
        scores.retain(|(_, score)| *score > 0.0);
        if scores.is_empty() {
            continue;
        }

        let length_bonus = if segment.len() > 500 {
            2.0
        } else if segment.len() > 200 {
            1.0
        } else {
            0.0
        };

        scores.sort_by(|left, right| right.1.total_cmp(&left.1));
        let (mut best_type, best_score) = scores[0];
        best_type = disambiguate(best_type, &prose, &scores);
        let confidence = (best_score + length_bonus).min(3.0) / 3.0;
        if confidence < min_confidence {
            continue;
        }

        memories.push(ConversationChunk {
            content: segment.trim().to_string(),
            room: best_type.to_string(),
            chunk_index: memories.len() as i32,
        });
    }

    memories
}

fn split_into_segments(text: &str) -> Vec<String> {
    let lines = text.lines().collect::<Vec<_>>();
    if count_turn_markers(&lines) >= 3 {
        let by_turn = split_by_turns(&lines)
            .into_iter()
            .map(|(_, segment)| segment)
            .collect::<Vec<_>>();
        if !by_turn.is_empty() {
            return by_turn;
        }
    }

    let paragraphs = split_paragraphs(text);
    if paragraphs.len() <= 1 && lines.len() > 20 {
        return lines
            .chunks(LINE_GROUP_SIZE)
            .map(|group| group.join("\n").trim().to_string())
            .filter(|group| !group.is_empty())
            .collect();
    }
    paragraphs
}

fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn extract_prose(text: &str) -> String {
    let mut prose = Vec::new();
    let mut in_code = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        if !is_code_line(line) {
            prose.push(line);
        }
    }
    let result = prose.join("\n").trim().to_string();
    if result.is_empty() {
        text.trim().to_string()
    } else {
        result
    }
}

fn is_code_line(line: &str) -> bool {
    let stripped = line.trim();
    if stripped.is_empty() {
        return false;
    }

    let patterns = [
        r"^\s*[\$#]\s",
        r"^\s*(cd|source|echo|export|pip|npm|git|python|bash|curl|wget|mkdir|rm|cp|mv|ls|cat|grep|find|chmod|sudo|brew|docker)\s",
        r"^\s*(import|from|def|class|function|const|let|var|return)\s",
        r"^\s*[A-Z_]{2,}=",
        r"^\s*\|",
        r"^\s*[-]{2,}",
        r"^\s*[{}\[\]]\s*$",
        r"^\s*(if|for|while|try|except|elif|else:)\b",
        r"^\s*\w+\.\w+\(",
        r"^\s*\w+\s*=\s*\w+\.\w+",
    ];
    if patterns
        .iter()
        .any(|pattern| Regex::new(pattern).is_ok_and(|re| re.is_match(stripped)))
    {
        return true;
    }

    let alpha = stripped
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .count();
    let ratio = alpha as f64 / stripped.len().max(1) as f64;
    ratio < 0.4 && stripped.len() > 10
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

fn split_by_turns(lines: &[&str]) -> Vec<(SpeakerRole, String)> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut current_role = SpeakerRole::Unknown;

    for line in lines {
        let trimmed = line.trim();
        let role = detect_speaker_role(trimmed);
        if role != SpeakerRole::Unknown && !current.is_empty() {
            segments.push((current_role, current.join("\n")));
            current = vec![(*line).to_string()];
            current_role = role;
        } else {
            if current.is_empty() {
                current_role = role;
            }
            current.push((*line).to_string());
        }
    }

    if !current.is_empty() {
        segments.push((current_role, current.join("\n")));
    }

    segments
}

fn count_turn_markers(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|line| detect_speaker_role(line.trim()) != SpeakerRole::Unknown)
        .count()
}

fn detect_speaker_role(line: &str) -> SpeakerRole {
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("> ") {
        return SpeakerRole::User;
    }
    if ["human:", "user:", "q:"]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return SpeakerRole::User;
    }
    if ["assistant:", "ai:", "a:", "claude:", "chatgpt:"]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return SpeakerRole::Assistant;
    }
    SpeakerRole::Unknown
}

#[cfg(test)]
mod tests {
    use super::extract_general_memories;

    #[test]
    fn general_extractor_classifies_all_memory_types() {
        let text = r#"
We decided to switch to LanceDB because the local-first trade-off is better.

I prefer keeping the hot path explicit and never hiding it behind convenience APIs.

The migration finally worked and we shipped the first prototype yesterday.

The build keeps failing with a database error, the deploy is blocked, and this bug is still broken.

I feel proud and grateful that the rewrite finally feels solid.
"#;

        let memories = extract_general_memories(text, 0.3);
        let kinds = memories
            .iter()
            .map(|chunk| chunk.room.as_str())
            .collect::<Vec<_>>();
        assert!(kinds.contains(&"decision"));
        assert!(kinds.contains(&"preference"));
        assert!(kinds.contains(&"milestone"));
        assert!(kinds.contains(&"problem"));
        assert!(kinds.contains(&"emotional"));
    }

    #[test]
    fn general_extractor_promotes_resolved_problem_to_milestone() {
        let text =
            "The deployment problem was painful, but we fixed it and now it works perfectly.";
        let memories = extract_general_memories(text, 0.3);
        assert_eq!(memories[0].room, "milestone");
    }

    #[test]
    fn general_extractor_keeps_positive_emotional_text_out_of_problem() {
        let text =
            "I feel grateful and proud that the difficult rewrite is finally stable and beautiful.";
        let memories = extract_general_memories(text, 0.3);
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].room, "emotional");
    }
}
