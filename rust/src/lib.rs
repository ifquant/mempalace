pub mod audit;
pub mod bootstrap;
pub mod config;
pub mod convo;
pub mod dialect;
pub mod embed;
pub mod error;
pub mod hook;
pub mod instructions;
pub mod mcp;
pub mod model;
pub mod service;
pub mod split;
pub mod storage;

pub use config::AppConfig;
pub use error::{MempalaceError, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
