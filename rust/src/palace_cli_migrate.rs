use anyhow::Result;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::MigrateSummary;
use serde_json::json;

use crate::cli_support::palace_exists;
use crate::palace_cli_maintenance_support::{create_app, print_json, resolve_config};

pub async fn handle_migrate(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
) -> Result<()> {
    let config = resolve_config(
        palace,
        hf_endpoint,
        human,
        print_migrate_error_human,
        print_migrate_error_json,
    )?;
    if human && !palace_exists(&config) {
        print_migrate_no_palace_human(&config);
        return Ok(());
    }
    let app = create_app(
        config,
        human,
        print_migrate_error_human,
        print_migrate_error_json,
    )?;
    let summary = match app.migrate().await {
        Ok(summary) => summary,
        Err(err) if human => {
            print_migrate_error_human(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_migrate_error_json(&err.to_string())?;
            std::process::exit(1);
        }
    };
    if human {
        print_migrate_human(&summary);
    } else {
        print_json(&summary)?;
    }
    Ok(())
}

fn print_migrate_human(summary: &MigrateSummary) {
    println!("\n{}", "=".repeat(60));
    println!("  MemPalace Migrate");
    println!("{}\n", "=".repeat(60));
    println!("  Palace:  {}", summary.palace_path);
    println!("  SQLite:  {}", summary.sqlite_path);
    println!(
        "  Before:  {}",
        summary
            .schema_version_before
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!("  After:   {}", summary.schema_version_after);
    if summary.changed {
        println!("\n  Migration complete.");
    } else {
        println!("\n  Nothing to migrate.");
    }
    println!("\n{}", "=".repeat(60));
    println!();
}

fn print_migrate_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
}

fn print_migrate_error_human(message: &str) {
    println!("\n  Migrate error: {message}");
    println!("  Check the palace SQLite file, then rerun `mempalace-rs migrate`.");
}

fn print_migrate_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Migrate error: {message}"),
        "hint": "Check the palace SQLite file, then rerun `mempalace-rs migrate`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}
