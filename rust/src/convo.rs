//! Public conversation-ingestion facade.
//!
//! This module exposes the transcript scanning and extraction entrypoints used
//! by conversation mining after normalization has produced plain-text content.

pub use crate::convo_exchange::{detect_convo_room, exchange_rooms, extract_exchange_chunks};
pub use crate::convo_general::{extract_general_memories, general_rooms};
pub use crate::convo_scan::scan_convo_files;

pub const MIN_CONVO_CHUNK_SIZE: usize = 30;

#[derive(Clone, Debug, PartialEq)]
/// A mined conversation chunk ready to become a drawer.
pub struct ConversationChunk {
    pub content: String,
    pub room: String,
    pub chunk_index: i32,
}
