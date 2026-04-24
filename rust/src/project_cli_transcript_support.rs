use anyhow::Result;

use crate::project_cli_support::print_json;

pub fn print_transcript_json<T: serde::Serialize>(value: &T) -> Result<()> {
    print_json(value)
}
