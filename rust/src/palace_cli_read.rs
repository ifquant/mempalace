use anyhow::Result;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::{
    CompressSummary, LayerStatusSummary, RecallSummary, Status, Taxonomy, WakeUpSummary,
};
use serde_json::json;

use crate::cli_support::{palace_exists, print_no_palace};
use crate::palace_cli_support::{create_app, print_json, resolve_config};

pub async fn handle_compress(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    wing: Option<String>,
    dry_run: bool,
    human: bool,
) -> Result<()> {
    let config = resolve_config(
        palace,
        hf_endpoint,
        human,
        print_compress_error_human,
        print_compress_error_json,
    )?;
    if !palace_exists(&config) {
        if human {
            print_compress_no_palace_human(&config);
        } else {
            print_no_palace(&config)?;
        }
        std::process::exit(1);
    }
    let app = create_app(
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
        print_json(&summary)?;
    }
    Ok(())
}

pub async fn handle_wake_up(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    wing: Option<String>,
    human: bool,
) -> Result<()> {
    let config = resolve_config(
        palace,
        hf_endpoint,
        human,
        print_wake_up_error_human,
        print_wake_up_error_json,
    )?;
    if !palace_exists(&config) {
        if human {
            print_wake_up_no_palace_human(&config);
        } else {
            print_no_palace(&config)?;
        }
        std::process::exit(1);
    }
    let app = create_app(
        config,
        human,
        print_wake_up_error_human,
        print_wake_up_error_json,
    )?;
    let summary = match app.wake_up(wing.as_deref()).await {
        Ok(summary) => summary,
        Err(err) if human => {
            print_wake_up_error_human(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_wake_up_error_json(&err.to_string())?;
            std::process::exit(1);
        }
    };
    if human {
        print_wake_up_human(&summary);
    } else {
        print_json(&summary)?;
    }
    Ok(())
}

pub async fn handle_recall(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    wing: Option<String>,
    room: Option<String>,
    results: usize,
    human: bool,
) -> Result<()> {
    let config = resolve_config(
        palace,
        hf_endpoint,
        human,
        print_recall_error_human,
        print_recall_error_json,
    )?;
    if !palace_exists(&config) {
        if human {
            print_recall_no_palace_human(&config);
        } else {
            print_no_palace(&config)?;
        }
        std::process::exit(1);
    }
    let app = create_app(
        config,
        human,
        print_recall_error_human,
        print_recall_error_json,
    )?;
    let summary = match app.recall(wing.as_deref(), room.as_deref(), results).await {
        Ok(summary) => summary,
        Err(err) if human => {
            print_recall_error_human(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_recall_error_json(&err.to_string())?;
            std::process::exit(1);
        }
    };
    if human {
        print_recall_human(&summary);
    } else {
        print_json(&summary)?;
    }
    Ok(())
}

pub async fn handle_layers_status(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
) -> Result<()> {
    let config = resolve_config(
        palace,
        hf_endpoint,
        human,
        print_layers_status_error_human,
        print_layers_status_error_json,
    )?;
    if !palace_exists(&config) {
        if human {
            print_layers_status_no_palace_human(&config);
        } else {
            print_no_palace(&config)?;
        }
        return Ok(());
    }
    let app = create_app(
        config,
        human,
        print_layers_status_error_human,
        print_layers_status_error_json,
    )?;
    let summary = match app.layer_status().await {
        Ok(summary) => summary,
        Err(err) if human => {
            print_layers_status_error_human(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_layers_status_error_json(&err.to_string())?;
            std::process::exit(1);
        }
    };
    if human {
        print_layers_status_human(&summary);
    } else {
        print_json(&summary)?;
    }
    Ok(())
}

pub async fn handle_status(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
) -> Result<()> {
    let config = resolve_config(
        palace,
        hf_endpoint,
        human,
        print_status_error_human,
        print_status_error_json,
    )?;
    if !palace_exists(&config) {
        if human {
            print_status_no_palace_human(&config);
        } else {
            print_no_palace(&config)?;
        }
        return Ok(());
    }
    let app = create_app(
        config,
        human,
        print_status_error_human,
        print_status_error_json,
    )?;
    let summary = match app.status().await {
        Ok(summary) => summary,
        Err(err) if human => {
            print_status_error_human(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_status_error_json(&err.to_string())?;
            std::process::exit(1);
        }
    };
    if human {
        let taxonomy = match app.taxonomy().await {
            Ok(taxonomy) => taxonomy,
            Err(err) => {
                print_status_error_human(&err.to_string());
                std::process::exit(1);
            }
        };
        print_status_human(&summary, &taxonomy);
    } else {
        print_json(&summary)?;
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

fn print_wake_up_human(summary: &WakeUpSummary) {
    println!("{}", summary.identity);
    println!();
    println!("{}", summary.layer1);
    println!();
    println!("Token estimate: {}", summary.token_estimate);
}

fn print_wake_up_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
    println!("  Run: mempalace init <dir> then mempalace mine <dir>");
}

fn print_wake_up_error_human(message: &str) {
    println!("\n  Wake-up error: {message}");
    println!("  Check the palace files, then rerun `mempalace-rs wake-up`.");
}

fn print_wake_up_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Wake-up error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs wake-up`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_recall_human(summary: &RecallSummary) {
    println!("{}", summary.text);
}

fn print_recall_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
    println!("  Run: mempalace-rs init <dir> && mempalace-rs mine <dir>");
}

fn print_recall_error_human(message: &str) {
    println!("\n  Recall error: {message}");
    println!("  Check the palace files, then rerun `mempalace-rs recall`.");
}

fn print_recall_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Recall error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs recall`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_layers_status_human(summary: &LayerStatusSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Layer Status");
    println!("{}\n", "=".repeat(55));
    println!("  Palace:  {}", summary.palace_path);
    println!("  SQLite:  {}", summary.sqlite_path);
    println!("  Drawers: {}", summary.total_drawers);
    println!(
        "  L0:      {}{}",
        summary.identity_path,
        if summary.identity_exists {
            " (present)"
        } else {
            " (missing)"
        }
    );
    println!("  L0 tokens: {}", summary.identity_tokens);
    println!("  L1: {}", summary.layer1_description);
    println!("  L2: {}", summary.layer2_description);
    println!("  L3: {}", summary.layer3_description);
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_layers_status_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
}

fn print_layers_status_error_human(message: &str) {
    println!("\n  Layers status error: {message}");
    println!("  Check the palace files, then rerun `mempalace-rs layers-status`.");
}

fn print_layers_status_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Layers status error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs layers-status`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_status_human(summary: &Status, taxonomy: &Taxonomy) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Status — {} drawers", summary.total_drawers);
    println!("{}\n", "=".repeat(55));
    if summary.total_drawers == 0 {
        println!("  Palace is initialized but still empty.");
        println!("  Run: mempalace mine <dir>");
        println!();
    }
    for (wing, rooms) in &taxonomy.taxonomy {
        println!("  WING: {wing}");
        let mut room_entries = rooms.iter().collect::<Vec<_>>();
        room_entries.sort_by(|(left_room, left_count), (right_room, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| left_room.cmp(right_room))
        });
        for (room, count) in room_entries {
            println!("    ROOM: {room:20} {count:5} drawers");
        }
        println!();
    }
    println!("{}", "=".repeat(55));
    println!();
}

fn print_status_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
    println!("  Run: mempalace init <dir> then mempalace mine <dir>");
}

fn print_status_error_human(message: &str) {
    println!("\n  Status error: {message}");
    println!("  Check the palace files, then rerun `mempalace-rs status`.");
}

fn print_status_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Status error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs status`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}
