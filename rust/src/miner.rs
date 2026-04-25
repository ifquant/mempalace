//! Mining entrypoints shared by the service layer.
//!
//! Audit readers can start here to see the two high-level ingest modes, then
//! follow `miner_project` for source-file mining or `miner_convo` for
//! transcript-oriented conversation mining.

pub use crate::miner_convo::mine_conversations_run;
pub use crate::miner_project::mine_project_run;
