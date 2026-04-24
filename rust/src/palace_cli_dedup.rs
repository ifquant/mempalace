use anyhow::Result;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::DedupSummary;
use serde_json::json;

use crate::cli_support::{palace_exists, print_no_palace};
use crate::palace_cli_maintenance_support::{create_app, print_json, resolve_config};

pub struct DedupCommand {
    pub threshold: f64,
    pub dry_run: bool,
    pub stats: bool,
    pub wing: Option<String>,
    pub source: Option<String>,
    pub human: bool,
}

pub async fn handle_dedup(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    command: DedupCommand,
) -> Result<()> {
    let config = resolve_config(
        palace,
        hf_endpoint,
        command.human,
        print_dedup_error_human,
        print_dedup_error_json,
    )?;
    if !palace_exists(&config) {
        if command.human {
            print_dedup_no_palace_human(&config);
        } else {
            print_no_palace(&config)?;
        }
        std::process::exit(1);
    }
    let app = create_app(
        config,
        command.human,
        print_dedup_error_human,
        print_dedup_error_json,
    )?;
    let summary = match app
        .dedup(
            command.threshold,
            command.dry_run,
            command.wing.as_deref(),
            command.source.as_deref(),
            5,
            command.stats,
        )
        .await
    {
        Ok(summary) => summary,
        Err(err) if command.human => {
            print_dedup_error_human(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_dedup_error_json(&err.to_string())?;
            std::process::exit(1);
        }
    };
    if command.human {
        print_dedup_human(&summary);
    } else {
        print_json(&summary)?;
    }
    Ok(())
}

fn print_dedup_human(summary: &DedupSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Deduplicator");
    println!("{}\n", "=".repeat(55));
    println!("  Palace: {}", summary.palace_path);
    println!("  Threshold: {}", summary.threshold);
    println!(
        "  Mode: {}",
        if summary.dry_run { "DRY RUN" } else { "LIVE" }
    );
    if let Some(wing) = &summary.wing {
        println!("  Wing: {wing}");
    }
    if let Some(source) = &summary.source {
        println!("  Source filter: {source}");
    }
    println!("  Sources checked: {}", summary.sources_checked);
    println!("  Drawers checked: {}", summary.total_drawers);
    println!("  Kept: {}", summary.kept);
    println!("  Deleted: {}", summary.deleted);
    if !summary.groups.is_empty() {
        println!("\n  Top groups:");
        for group in summary.groups.iter().take(15) {
            println!(
                "    {}  {} -> {} (-{})",
                group.source_file, group.before, group.kept, group.deleted
            );
        }
    }
    if summary.stats_only {
        println!("\n  Stats-only mode: no changes were made.");
    } else if summary.dry_run {
        println!("\n  [DRY RUN] No changes written. Re-run without --dry-run to apply.");
    }
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_dedup_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
}

fn print_dedup_error_human(message: &str) {
    println!("\n  Dedup error: {message}");
    println!("  Check the palace files, then rerun `mempalace-rs dedup`.");
}

fn print_dedup_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Dedup error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs dedup`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}
