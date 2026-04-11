pub mod config;
pub mod embed;
pub mod error;
pub mod mcp;
pub mod model;
pub mod service;
pub mod storage;

pub use config::AppConfig;
pub use error::{MempalaceError, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
