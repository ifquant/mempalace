use crate::error::{MempalaceError, Result};

pub fn render(name: &str) -> Result<&'static str> {
    match name {
        "help" => Ok(include_str!("../instructions/help.md")),
        "init" => Ok(include_str!("../instructions/init.md")),
        "mine" => Ok(include_str!("../instructions/mine.md")),
        "search" => Ok(include_str!("../instructions/search.md")),
        "status" => Ok(include_str!("../instructions/status.md")),
        other => Err(MempalaceError::InvalidArgument(format!(
            "Unknown instructions: {other}"
        ))),
    }
}
