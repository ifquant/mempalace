use crate::convo::{ConversationChunk, MIN_CONVO_CHUNK_SIZE};

use super::rooms::detect_convo_room;

const LINE_GROUP_SIZE: usize = 25;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SpeakerRole {
    User,
    Assistant,
    Unknown,
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

fn split_paragraphs(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
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
