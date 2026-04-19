use std::path::{Path, PathBuf};

use anyhow::Result;
use mempalace_rs::normalize::normalize_conversation_file;
use mempalace_rs::split;
use serde_json::{Value, json};

use crate::project_cli_support::print_json;

pub fn handle_split(
    dir: &Path,
    output_dir: Option<&Path>,
    min_sessions: usize,
    dry_run: bool,
) -> Result<()> {
    let summary = split::split_directory(dir, output_dir, min_sessions, dry_run)?;
    print_json(&summary)
}

pub fn handle_normalize(file: &PathBuf, human: bool) -> Result<()> {
    let raw = std::fs::read_to_string(file)?;
    let normalized = normalize_conversation_file(file)?;
    let Some(normalized) = normalized else {
        if human {
            print_normalize_error_human("Unsupported or unreadable conversation file.");
        } else {
            print_json(&json!({
                "error": "Normalize error: Unsupported or unreadable conversation file."
            }))?;
        }
        std::process::exit(1);
    };
    let summary = json!({
        "kind": "normalize",
        "file_path": file.display().to_string(),
        "changed": normalized != raw,
        "chars": normalized.chars().count(),
        "quote_turns": normalized.lines().filter(|line| line.trim_start().starts_with('>')).count(),
        "normalized": normalized,
    });
    if human {
        print_normalize_human(&summary);
    } else {
        print_json(&summary)?;
    }
    Ok(())
}

fn print_normalize_human(summary: &Value) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Normalize");
    println!("{}\n", "=".repeat(55));
    println!(
        "  File: {}",
        summary["file_path"].as_str().unwrap_or_default()
    );
    println!(
        "  Changed: {}",
        summary["changed"].as_bool().unwrap_or(false)
    );
    println!("  Chars: {}", summary["chars"].as_u64().unwrap_or(0));
    println!(
        "  User turns: {}",
        summary["quote_turns"].as_u64().unwrap_or(0)
    );
    println!("\n  Preview:\n");
    let preview = summary["normalized"]
        .as_str()
        .unwrap_or_default()
        .lines()
        .take(12)
        .collect::<Vec<_>>()
        .join("\n");
    println!("{preview}");
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_normalize_error_human(message: &str) {
    println!("\n  Normalize error: {message}");
}
