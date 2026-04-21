use std::path::{Path, PathBuf};

use anyhow::Result;
use mempalace_rs::split;

use crate::project_cli_transcript_support::print_transcript_json;

pub fn handle_split(
    dir: Option<&Path>,
    source: Option<&Path>,
    file: Option<&Path>,
    output_dir: Option<&Path>,
    min_sessions: usize,
    dry_run: bool,
) -> Result<()> {
    let summary = if let Some(file) = file {
        split::split_single_file(file, output_dir, min_sessions, dry_run)?
    } else {
        let source_dir = dir
            .or(source)
            .map(Path::to_path_buf)
            .unwrap_or_else(default_split_source_dir);
        split::split_directory(&source_dir, output_dir, min_sessions, dry_run)?
    };
    print_transcript_json(&summary)
}

fn default_split_source_dir() -> PathBuf {
    std::env::var_os("MEMPALACE_SOURCE_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Desktop/transcripts")
        })
}
