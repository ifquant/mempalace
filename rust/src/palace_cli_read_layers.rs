use anyhow::Result;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::{LayerStatusSummary, RecallSummary, WakeUpSummary};
use serde_json::json;

use crate::palace_cli_read_support::{
    create_read_app, exit_if_no_palace_human_or_json, print_read_json, resolve_read_config,
};

pub async fn handle_wake_up(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    wing: Option<String>,
    human: bool,
) -> Result<()> {
    let config = resolve_read_config(
        palace,
        hf_endpoint,
        human,
        print_wake_up_error_human,
        print_wake_up_error_json,
    )?;
    exit_if_no_palace_human_or_json(&config, human, print_wake_up_no_palace_human)?;
    let app = create_read_app(
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
        print_read_json(&summary)?;
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
    let config = resolve_read_config(
        palace,
        hf_endpoint,
        human,
        print_recall_error_human,
        print_recall_error_json,
    )?;
    exit_if_no_palace_human_or_json(&config, human, print_recall_no_palace_human)?;
    let app = create_read_app(
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
        print_read_json(&summary)?;
    }
    Ok(())
}

pub async fn handle_layers_status(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
) -> Result<()> {
    let config = resolve_read_config(
        palace,
        hf_endpoint,
        human,
        print_layers_status_error_human,
        print_layers_status_error_json,
    )?;
    if !crate::cli_support::palace_exists(&config) {
        if human {
            print_layers_status_no_palace_human(&config);
        } else {
            crate::cli_support::print_no_palace(&config)?;
        }
        return Ok(());
    }
    let app = create_read_app(
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
        print_read_json(&summary)?;
    }
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
