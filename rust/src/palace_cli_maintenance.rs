use anyhow::Result;
use clap::Subcommand;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::{
    DedupSummary, MigrateSummary, RepairPruneSummary, RepairRebuildSummary, RepairScanSummary,
    RepairSummary,
};
use serde_json::json;

use crate::cli_support::{palace_exists, print_no_palace};
use crate::palace_cli_support::{create_app, print_json, resolve_config};

#[derive(Subcommand, Clone)]
pub enum RepairCommand {
    #[command(about = "Scan for vector/SQLite drift and write corrupt_ids.txt")]
    Scan {
        #[arg(long)]
        #[arg(help = "Scan only this wing")]
        wing: Option<String>,
    },
    #[command(about = "Delete IDs listed in corrupt_ids.txt")]
    Prune {
        #[arg(long)]
        #[arg(help = "Actually delete the queued IDs")]
        confirm: bool,
    },
    #[command(about = "Rebuild the vector store from SQLite drawers")]
    Rebuild,
}

pub struct DedupCommand {
    pub threshold: f64,
    pub dry_run: bool,
    pub stats: bool,
    pub wing: Option<String>,
    pub source: Option<String>,
    pub human: bool,
}

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

pub async fn handle_repair(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    action: Option<RepairCommand>,
    human: bool,
) -> Result<()> {
    let config = resolve_config(
        palace,
        hf_endpoint,
        human,
        print_repair_error_human,
        print_repair_error_json,
    )?;
    if human && !palace_exists(&config) {
        print_repair_no_palace_human(&config);
        return Ok(());
    }
    let app = create_app(
        config,
        human,
        print_repair_error_human,
        print_repair_error_json,
    )?;
    match action {
        None => {
            let summary = match app.repair().await {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_repair_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_repair_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_repair_human(&summary);
            } else {
                print_json(&summary)?;
            }
        }
        Some(RepairCommand::Scan { wing }) => {
            let summary = match app.repair_scan(wing.as_deref()).await {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_repair_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_repair_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_repair_scan_human(&summary);
            } else {
                print_json(&summary)?;
            }
        }
        Some(RepairCommand::Prune { confirm }) => {
            let summary = match app.repair_prune(confirm).await {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_repair_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_repair_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_repair_prune_human(&summary);
            } else {
                print_json(&summary)?;
            }
        }
        Some(RepairCommand::Rebuild) => {
            let summary = match app.repair_rebuild().await {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_repair_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_repair_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_repair_rebuild_human(&summary);
            } else {
                print_json(&summary)?;
            }
        }
    }
    Ok(())
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

fn print_repair_human(summary: &RepairSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Repair");
    println!("{}\n", "=".repeat(55));
    println!("  Palace: {}", summary.palace_path);
    println!(
        "  SQLite: {}",
        if summary.sqlite_exists {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "  LanceDB: {}",
        if summary.lance_exists {
            "present"
        } else {
            "missing"
        }
    );
    if let Some(drawers) = summary.sqlite_drawer_count {
        println!("  Drawers found: {drawers}");
    }
    if let Some(version) = summary.schema_version {
        println!("  Schema version: {version}");
    }
    if let Some(provider) = &summary.embedding_provider {
        let model = summary.embedding_model.as_deref().unwrap_or("unknown");
        let dimension = summary
            .embedding_dimension
            .map(|value| value.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!("  Embedding: {provider}/{model}/{dimension}");
    }
    println!(
        "  Vector access: {}",
        if summary.vector_accessible {
            "ok"
        } else {
            "failed"
        }
    );
    if summary.issues.is_empty() {
        println!("\n  Repair diagnostics look healthy.");
    } else {
        println!("\n  Repair diagnostics found problems.");
        println!("\n  Issues:");
        for issue in &summary.issues {
            println!("    - {issue}");
        }
        println!("\n  Suggested next step:");
        println!(
            "    Fix the missing or mismatched palace components, then rerun `mempalace-rs repair`."
        );
    }
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_repair_scan_human(summary: &RepairScanSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Repair Scan");
    println!("{}\n", "=".repeat(55));
    println!("  Palace: {}", summary.palace_path);
    if let Some(wing) = &summary.wing {
        println!("  Wing: {wing}");
    }
    println!("  SQLite drawers: {}", summary.sqlite_drawers);
    println!("  Vector drawers: {}", summary.vector_drawers);
    println!(
        "  Missing from vector: {}",
        summary.missing_from_vector.len()
    );
    println!("  Orphaned in vector: {}", summary.orphaned_in_vector.len());
    println!("  corrupt_ids.txt: {}", summary.corrupt_ids_path);
    if !summary.missing_from_vector.is_empty() {
        println!("\n  SQLite drawers missing from vector:");
        for drawer_id in summary.missing_from_vector.iter().take(10) {
            println!("    - {drawer_id}");
        }
    }
    if !summary.orphaned_in_vector.is_empty() {
        println!("\n  Vector-only drawers queued for prune:");
        for drawer_id in summary.orphaned_in_vector.iter().take(10) {
            println!("    - {drawer_id}");
        }
    }
    println!("\n  Suggested next step:");
    if !summary.missing_from_vector.is_empty() {
        println!("    Run `mempalace-rs repair rebuild` to restore the vector store.");
    } else if summary.prune_candidates > 0 {
        println!("    Run `mempalace-rs repair prune --confirm` to delete queued vector orphans.");
    } else {
        println!("    No repair actions are currently needed.");
    }
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_repair_prune_human(summary: &RepairPruneSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Repair Prune");
    println!("{}\n", "=".repeat(55));
    println!("  Palace: {}", summary.palace_path);
    println!("  corrupt_ids.txt: {}", summary.corrupt_ids_path);
    println!("  Queued: {}", summary.queued);
    println!(
        "  Mode: {}",
        if summary.confirm { "LIVE" } else { "DRY RUN" }
    );
    println!("  Deleted from vector: {}", summary.deleted_from_vector);
    println!("  Deleted from sqlite: {}", summary.deleted_from_sqlite);
    if !summary.confirm {
        println!("\n  Re-run with `mempalace-rs repair prune --confirm` to apply.");
    }
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_repair_rebuild_human(summary: &RepairRebuildSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Repair Rebuild");
    println!("{}\n", "=".repeat(55));
    println!("  Palace: {}", summary.palace_path);
    println!("  Drawers found: {}", summary.drawers_found);
    println!("  Rebuilt: {}", summary.rebuilt);
    if let Some(backup_path) = &summary.backup_path {
        println!("  SQLite backup: {backup_path}");
    }
    println!("\n  Vector store rebuilt from SQLite source of truth.");
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_repair_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
}

fn print_repair_error_human(message: &str) {
    println!("\n  Repair error: {message}");
    println!("  Check the palace files, then rerun `mempalace-rs repair`.");
}

fn print_repair_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Repair error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs repair`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
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
