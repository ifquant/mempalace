//! General-memory extraction for normalized conversations.
//!
//! Unlike exchange extraction, this path tries to pull standalone memories such
//! as decisions or problems out of longer transcript text.

use crate::convo::ConversationChunk;

#[path = "convo_general_scoring.rs"]
mod scoring;
#[path = "convo_general_segments.rs"]
mod segments;

const GENERAL_TYPES: &[&str] = &[
    "decision",
    "preference",
    "milestone",
    "problem",
    "emotional",
];

/// Returns the room set used by general-memory extraction.
pub fn general_rooms() -> Vec<String> {
    GENERAL_TYPES
        .iter()
        .map(|item| (*item).to_string())
        .collect()
}

/// Extracts memory-like conversation segments above the configured confidence.
///
/// This pipeline intentionally ignores turn pairing and instead classifies
/// segments by semantic type after prose cleanup.
pub fn extract_general_memories(text: &str, min_confidence: f64) -> Vec<ConversationChunk> {
    let segments = segments::split_into_segments(text);
    let mut memories = Vec::new();

    for segment in segments {
        if segment.trim().len() < 20 {
            continue;
        }

        let prose = segments::extract_prose(&segment);
        let Some((best_type, best_score)) = scoring::score_segment(&prose) else {
            continue;
        };
        let confidence = scoring::confidence(best_score, segment.len());
        if confidence < min_confidence {
            continue;
        }

        memories.push(ConversationChunk {
            content: segment.trim().to_string(),
            room: best_type,
            chunk_index: memories.len() as i32,
        });
    }

    memories
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
