use std::path::Path;

use anyhow::Result;
use mempalace_rs::split;

use crate::project_cli_transcript_support::print_transcript_json;

pub fn handle_split(
    dir: &Path,
    output_dir: Option<&Path>,
    min_sessions: usize,
    dry_run: bool,
) -> Result<()> {
    let summary = split::split_directory(dir, output_dir, min_sessions, dry_run)?;
    print_transcript_json(&summary)
}
