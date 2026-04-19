use anyhow::Result;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::CompressSummary;
use serde_json::json;

use crate::palace_cli_read_support::{
    create_read_app, exit_if_no_palace_human_or_json, print_read_json, resolve_read_config,
};

pub async fn handle_compress(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    wing: Option<String>,
    dry_run: bool,
    human: bool,
) -> Result<()> {
    let config = resolve_read_config(
        palace,
        hf_endpoint,
        human,
        print_compress_error_human,
        print_compress_error_json,
    )?;
    exit_if_no_palace_human_or_json(&config, human, print_compress_no_palace_human)?;
    let app = create_read_app(
        config,
        human,
        print_compress_error_human,
        print_compress_error_json,
    )?;
    let summary = match app.compress(wing.as_deref(), dry_run).await {
        Ok(summary) => summary,
        Err(err) if human => {
            print_compress_error_human(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_compress_error_json(&err.to_string())?;
            std::process::exit(1);
        }
    };
    if human {
        print_compress_human(&summary);
    } else {
        print_read_json(&summary)?;
    }
    Ok(())
}

fn print_compress_human(summary: &CompressSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Compress");
    println!("{}\n", "=".repeat(55));
    println!("  Palace:   {}", summary.palace_path);
    println!("  SQLite:   {}", summary.sqlite_path);
    println!("  Wing:     {}", summary.wing.as_deref().unwrap_or("all"));
    println!("  Processed: {}", summary.processed);
    println!("  Stored:    {}", summary.stored);
    println!(
        "  Tokens:    {} -> {} ({:.1}x)",
        summary.original_tokens, summary.compressed_tokens, summary.compression_ratio
    );
    if summary.dry_run {
        println!("\n  DRY RUN preview:");
        for entry in summary.entries.iter().take(3) {
            println!(
                "    [{} / {}] {}",
                entry.wing, entry.room, entry.source_file
            );
            for line in entry.aaak.lines() {
                println!("      {line}");
            }
        }
    } else {
        println!("\n  AAAK summaries stored in SQLite table `compressed_drawers`.");
    }
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_compress_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
    println!("  Run: mempalace init <dir> then mempalace mine <dir>");
}

fn print_compress_error_human(message: &str) {
    println!("\n  Compress error: {message}");
    println!("  Check the palace files, then rerun `mempalace-rs compress`.");
}

fn print_compress_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Compress error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs compress`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}
