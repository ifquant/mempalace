use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use regex::Regex;
use serde_json::Value;

use crate::error::Result;

const CONVO_EXTENSIONS: &[&str] = &[".txt", ".md", ".json", ".jsonl"];
const MAX_CONVO_FILE_SIZE: u64 = 10 * 1024 * 1024;
pub const MIN_CONVO_CHUNK_SIZE: usize = 30;
const LINE_GROUP_SIZE: usize = 25;

const TOPIC_BUCKETS: &[(&str, &[&str])] = &[
    (
        "technical",
        &[
            "code", "python", "function", "bug", "error", "api", "database", "server", "deploy",
            "git", "test", "debug", "refactor",
        ],
    ),
    (
        "architecture",
        &[
            "architecture",
            "design",
            "pattern",
            "structure",
            "schema",
            "interface",
            "module",
            "component",
            "service",
            "layer",
        ],
    ),
    (
        "planning",
        &[
            "plan",
            "roadmap",
            "milestone",
            "deadline",
            "priority",
            "sprint",
            "backlog",
            "scope",
            "requirement",
            "spec",
        ],
    ),
    (
        "decisions",
        &[
            "decided",
            "chose",
            "picked",
            "switched",
            "migrated",
            "replaced",
            "trade-off",
            "alternative",
            "option",
            "approach",
        ],
    ),
    (
        "problems",
        &[
            "problem",
            "issue",
            "broken",
            "failed",
            "crash",
            "stuck",
            "workaround",
            "fix",
            "solved",
            "resolved",
        ],
    ),
];

const GENERAL_TYPES: &[&str] = &[
    "decision",
    "preference",
    "milestone",
    "problem",
    "emotional",
];

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

#[derive(Clone, Debug, PartialEq)]
pub struct ConversationChunk {
    pub content: String,
    pub room: String,
    pub chunk_index: i32,
}

pub fn exchange_rooms() -> Vec<String> {
    TOPIC_BUCKETS
        .iter()
        .map(|(room, _)| (*room).to_string())
        .chain(std::iter::once("general".to_string()))
        .collect()
}

pub fn general_rooms() -> Vec<String> {
    GENERAL_TYPES
        .iter()
        .map(|item| (*item).to_string())
        .collect()
}

pub fn scan_convo_files(
    dir: &Path,
    respect_gitignore: bool,
    include_ignored: &[String],
    skip_dirs: &[&str],
) -> Result<Vec<PathBuf>> {
    let skip_dirs = skip_dirs
        .iter()
        .map(|item| (*item).to_string())
        .collect::<HashSet<_>>();
    let include_paths = normalize_include_paths(include_ignored);
    let include_paths_for_filter = include_paths.clone();
    let project_root = dir.to_path_buf();
    let mut builder = WalkBuilder::new(dir);
    builder.hidden(false);
    builder.git_ignore(respect_gitignore);
    builder.git_global(respect_gitignore);
    builder.git_exclude(respect_gitignore);
    builder.require_git(false);
    builder.filter_entry(move |entry| {
        if is_force_include(entry.path(), &project_root, &include_paths_for_filter) {
            return true;
        }

        entry
            .file_name()
            .to_str()
            .map(|name| !skip_dirs.contains(name) && !name.ends_with(".egg-info"))
            .unwrap_or(true)
    });

    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for result in builder.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() || path.is_symlink() {
            continue;
        }

        let exact_force_include = is_exact_force_include(path, dir, &include_paths);
        if !exact_force_include && should_skip_convo_file(path) {
            continue;
        }

        let stat = match path.metadata() {
            Ok(stat) => stat,
            Err(_) => continue,
        };
        if stat.len() > MAX_CONVO_FILE_SIZE {
            continue;
        }

        if seen.insert(path.to_path_buf()) {
            files.push(path.to_path_buf());
        }
    }

    for rel in include_ignored {
        let path = dir.join(rel);
        if path.is_file() && seen.insert(path.clone()) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

pub fn normalize_conversation_file(path: &Path) -> Result<Option<String>> {
    let raw = match fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => return Ok(None),
        },
        Err(err) => return Err(err.into()),
    };
    normalize_conversation(path, &raw)
}

pub fn normalize_conversation(path: &Path, raw: &str) -> Result<Option<String>> {
    let content = raw.trim();
    if content.is_empty() {
        return Ok(None);
    }

    if count_quote_lines(content) >= 3 {
        return Ok(Some(raw.to_string()));
    }

    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    if matches!(ext.as_str(), "json" | "jsonl")
        || matches!(content.chars().next(), Some('{') | Some('['))
    {
        if let Some(normalized) = try_normalize_json(content) {
            return Ok(Some(normalized));
        }
        if matches!(ext.as_str(), "json" | "jsonl") {
            return Ok(None);
        }
    }

    Ok(Some(raw.to_string()))
}

pub fn extract_exchange_chunks(text: &str) -> Vec<ConversationChunk> {
    let lines = text.lines().collect::<Vec<_>>();
    if count_quote_lines(text) >= 3 {
        return chunk_by_quote_exchange(&lines);
    }

    if count_turn_markers(&lines) >= 3 {
        let chunks = chunk_by_speaker_exchange(&lines);
        if !chunks.is_empty() {
            return chunks;
        }
    }

    chunk_by_paragraph(text)
}

pub fn detect_convo_room(content: &str) -> String {
    let content_lower = content
        .chars()
        .take(3_000)
        .collect::<String>()
        .to_ascii_lowercase();
    let mut best_room = "general";
    let mut best_score = 0usize;
    for (room, keywords) in TOPIC_BUCKETS {
        let score = keywords
            .iter()
            .map(|keyword| content_lower.matches(keyword).count())
            .sum::<usize>();
        if score > best_score {
            best_score = score;
            best_room = room;
        }
    }
    best_room.to_string()
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

fn chunk_by_quote_exchange(lines: &[&str]) -> Vec<ConversationChunk> {
    let mut chunks = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index].trim();
        if line.starts_with('>') {
            let user_turn = line.to_string();
            index += 1;
            let mut ai_lines = Vec::new();

            while index < lines.len() {
                let next = lines[index].trim();
                if next.starts_with('>') || next.starts_with("---") {
                    break;
                }
                if !next.is_empty() {
                    ai_lines.push(next.to_string());
                }
                index += 1;
            }

            let ai_response = ai_lines.into_iter().take(8).collect::<Vec<_>>().join(" ");
            let content = if ai_response.is_empty() {
                user_turn
            } else {
                format!("{user_turn}\n{ai_response}")
            };
            if content.trim().len() >= MIN_CONVO_CHUNK_SIZE {
                chunks.push(ConversationChunk {
                    room: detect_convo_room(&content),
                    content,
                    chunk_index: chunks.len() as i32,
                });
            }
        } else {
            index += 1;
        }
    }

    chunks
}

fn chunk_by_speaker_exchange(lines: &[&str]) -> Vec<ConversationChunk> {
    let segments = split_by_turns(lines);
    let mut chunks = Vec::new();
    let mut index = 0usize;

    while index < segments.len() {
        let (role, current) = &segments[index];
        if *role == SpeakerRole::User {
            let mut content = current.clone();
            if let Some((SpeakerRole::Assistant, response)) = segments.get(index + 1) {
                content.push('\n');
                content.push_str(response);
                index += 1;
            }
            if content.trim().len() >= MIN_CONVO_CHUNK_SIZE {
                chunks.push(ConversationChunk {
                    room: detect_convo_room(&content),
                    content,
                    chunk_index: chunks.len() as i32,
                });
            }
        }
        index += 1;
    }

    chunks
}

fn chunk_by_paragraph(text: &str) -> Vec<ConversationChunk> {
    let paragraphs = split_paragraphs(text);
    let parts = if paragraphs.len() <= 1 && text.lines().count() > 20 {
        text.lines()
            .collect::<Vec<_>>()
            .chunks(LINE_GROUP_SIZE)
            .map(|group| group.join("\n").trim().to_string())
            .filter(|group| !group.is_empty())
            .collect::<Vec<_>>()
    } else {
        paragraphs
    };

    parts
        .into_iter()
        .filter(|part| part.len() >= MIN_CONVO_CHUNK_SIZE)
        .enumerate()
        .map(|(index, content)| ConversationChunk {
            room: detect_convo_room(&content),
            content,
            chunk_index: index as i32,
        })
        .collect()
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Sentiment {
    Positive,
    Negative,
    Neutral,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SpeakerRole {
    User,
    Assistant,
    Unknown,
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

fn count_quote_lines(text: &str) -> usize {
    text.lines()
        .filter(|line| line.trim_start().starts_with("> "))
        .count()
}

fn should_skip_convo_file(path: &Path) -> bool {
    let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    if filename.ends_with(".meta.json") {
        return true;
    }
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext.to_ascii_lowercase()))
        .unwrap_or_default();
    !CONVO_EXTENSIONS.iter().any(|candidate| *candidate == ext)
}

fn normalize_include_paths(include_ignored: &[String]) -> HashSet<String> {
    include_ignored
        .iter()
        .map(|raw| raw.trim().trim_matches('/'))
        .filter(|raw| !raw.is_empty())
        .map(|raw| Path::new(raw).to_string_lossy().replace('\\', "/"))
        .collect()
}

fn is_exact_force_include(
    path: &Path,
    project_path: &Path,
    include_paths: &HashSet<String>,
) -> bool {
    if include_paths.is_empty() {
        return false;
    }
    path.strip_prefix(project_path)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .is_some_and(|relative| include_paths.contains(relative.trim_matches('/')))
}

fn is_force_include(path: &Path, project_path: &Path, include_paths: &HashSet<String>) -> bool {
    if include_paths.is_empty() {
        return false;
    }
    path.strip_prefix(project_path)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .is_some_and(|relative| {
            let relative = relative.trim_matches('/');
            include_paths
                .iter()
                .any(|include| relative == include || relative.starts_with(&format!("{include}/")))
        })
}

fn try_normalize_json(content: &str) -> Option<String> {
    if let Some(transcript) = try_claude_code_jsonl(content) {
        return Some(transcript);
    }
    if let Some(transcript) = try_codex_jsonl(content) {
        return Some(transcript);
    }

    let data: Value = serde_json::from_str(content).ok()?;
    try_flat_messages_json(&data)
        .or_else(|| try_claude_ai_json(&data))
        .or_else(|| try_chatgpt_json(&data))
        .or_else(|| try_slack_json(&data))
}

fn try_claude_code_jsonl(content: &str) -> Option<String> {
    let mut messages = Vec::new();
    for line in content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let entry: Value = serde_json::from_str(line).ok()?;
        let msg_type = entry
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let message = entry.get("message").unwrap_or(&Value::Null);
        match msg_type {
            "human" | "user" => {
                let text = extract_content(message.get("content").unwrap_or(&Value::Null));
                if !text.is_empty() {
                    messages.push(("user", text));
                }
            }
            "assistant" => {
                let text = extract_content(message.get("content").unwrap_or(&Value::Null));
                if !text.is_empty() {
                    messages.push(("assistant", text));
                }
            }
            _ => {}
        }
    }
    (messages.len() >= 2).then(|| messages_to_transcript(&messages))
}

fn try_codex_jsonl(content: &str) -> Option<String> {
    let mut messages = Vec::new();
    let mut has_session_meta = false;
    for line in content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let entry: Value = serde_json::from_str(line).ok()?;
        let entry_type = entry
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if entry_type == "session_meta" {
            has_session_meta = true;
            continue;
        }
        if entry_type != "event_msg" {
            continue;
        }
        let payload = entry.get("payload")?;
        let payload_type = payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let text = payload
            .get("message")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        if text.is_empty() {
            continue;
        }
        match payload_type {
            "user_message" => messages.push(("user", text.to_string())),
            "agent_message" => messages.push(("assistant", text.to_string())),
            _ => {}
        }
    }
    (messages.len() >= 2 && has_session_meta).then(|| messages_to_transcript(&messages))
}

fn try_flat_messages_json(data: &Value) -> Option<String> {
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
    (messages.len() >= 2).then(|| messages_to_transcript(&messages))
}

fn try_claude_ai_json(data: &Value) -> Option<String> {
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
        return (all_messages.len() >= 2).then(|| messages_to_transcript(&all_messages));
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
    (messages.len() >= 2).then(|| messages_to_transcript(&messages))
}

fn try_chatgpt_json(data: &Value) -> Option<String> {
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

    (messages.len() >= 2).then(|| messages_to_transcript(&messages))
}

fn try_slack_json(data: &Value) -> Option<String> {
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

    (messages.len() >= 2).then(|| messages_to_transcript(&messages))
}

fn extract_content(value: &Value) -> String {
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

fn messages_to_transcript(messages: &[(&str, String)]) -> String {
    let mut lines = Vec::new();
    let mut index = 0usize;
    while index < messages.len() {
        let (role, text) = &messages[index];
        if *role == "user" {
            lines.push(format!("> {}", text.trim()));
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

#[cfg(test)]
mod tests {
    use super::{detect_convo_room, extract_exchange_chunks, extract_general_memories};

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
    fn exchange_chunker_pairs_speaker_turns() {
        let text = "Human: why did the build fail?\nAssistant: The SQL schema was missing.\nHuman: how did we fix it?\nAssistant: We added the migration and reran tests.";
        let chunks = extract_exchange_chunks(text);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].content.contains("Human: why did the build fail?"));
        assert!(
            chunks[0]
                .content
                .contains("Assistant: The SQL schema was missing.")
        );
    }

    #[test]
    fn convo_room_detection_matches_python_keyword_buckets() {
        assert_eq!(
            detect_convo_room("We should update the roadmap and backlog before the next sprint."),
            "planning"
        );
        assert_eq!(
            detect_convo_room("The service architecture and module design changed."),
            "architecture"
        );
    }

    #[test]
    fn exchange_chunker_falls_back_to_paragraph_groups() {
        let text = "We reviewed the migration strategy and kept the old data path for safety.\n\nThis paragraph explains why the deploy failed and what changed in the build.\n\nThe final paragraph describes the testing follow-up and release plan.";
        let chunks = extract_exchange_chunks(text);
        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().all(|chunk| !chunk.content.is_empty()));
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
