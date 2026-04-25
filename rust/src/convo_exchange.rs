//! Exchange-style conversation extraction.
//!
//! This split facade groups the heuristics that keep conversational turn
//! structure and the lightweight room detector used for those chunks.

#[path = "convo_exchange_chunking.rs"]
mod chunking;
#[path = "convo_exchange_rooms.rs"]
mod rooms;

pub use chunking::extract_exchange_chunks;
pub use rooms::{detect_convo_room, exchange_rooms};

#[cfg(test)]
mod tests {
    use super::{detect_convo_room, extract_exchange_chunks};

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
}
