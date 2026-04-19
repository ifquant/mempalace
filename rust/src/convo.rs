pub use crate::convo_exchange::{detect_convo_room, exchange_rooms, extract_exchange_chunks};
pub use crate::convo_general::{extract_general_memories, general_rooms};
pub use crate::convo_scan::scan_convo_files;

pub const MIN_CONVO_CHUNK_SIZE: usize = 30;

#[derive(Clone, Debug, PartialEq)]
pub struct ConversationChunk {
    pub content: String,
    pub room: String,
    pub chunk_index: i32,
}
