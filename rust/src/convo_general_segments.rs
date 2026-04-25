use regex::Regex;

const LINE_GROUP_SIZE: usize = 25;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SpeakerRole {
    User,
    Assistant,
    Unknown,
}

/// Splits normalized transcript text into segments suitable for scoring.
pub fn split_into_segments(text: &str) -> Vec<String> {
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

/// Removes obvious code and shell fragments before general-memory scoring.
pub fn extract_prose(text: &str) -> String {
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

fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
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
