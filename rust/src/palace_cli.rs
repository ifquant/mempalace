use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::{
    CompressSummary, DedupSummary, DoctorSummary, LayerStatusSummary, MigrateSummary,
    PrepareEmbeddingSummary, RecallSummary, RepairPruneSummary, RepairRebuildSummary,
    RepairScanSummary, RepairSummary, Status, Taxonomy, WakeUpSummary,
};
use mempalace_rs::service::App;
use serde_json::json;

use crate::{apply_cli_overrides, palace_exists, print_no_palace};

pub enum PalaceCommand {
    Compress {
        wing: Option<String>,
        dry_run: bool,
        human: bool,
    },
    WakeUp {
        wing: Option<String>,
        human: bool,
    },
    Recall {
        wing: Option<String>,
        room: Option<String>,
        results: usize,
        human: bool,
    },
    LayersStatus {
        human: bool,
    },
    Migrate {
        human: bool,
    },
    Repair {
        action: Option<RepairCommand>,
        human: bool,
    },
    Dedup {
        threshold: f64,
        dry_run: bool,
        stats: bool,
        wing: Option<String>,
        source: Option<String>,
        human: bool,
    },
    Status {
        human: bool,
    },
    Doctor {
        warm_embedding: bool,
        human: bool,
    },
    PrepareEmbedding {
        attempts: usize,
        wait_ms: u64,
        human: bool,
    },
}

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

pub async fn handle_palace_command(
    command: PalaceCommand,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> Result<()> {
    match command {
        PalaceCommand::Compress {
            wing,
            dry_run,
            human,
        } => {
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
        }
        PalaceCommand::WakeUp { wing, human } => {
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
        }
        PalaceCommand::Recall {
            wing,
            room,
            results,
            human,
        } => {
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
        }
        PalaceCommand::LayersStatus { human } => {
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
        }
        PalaceCommand::Migrate { human } => {
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
        }
        PalaceCommand::Repair { action, human } => {
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
        }
        PalaceCommand::Dedup {
            threshold,
            dry_run,
            stats,
            wing,
            source,
            human,
        } => {
            let config = resolve_config(
                palace,
                hf_endpoint,
                human,
                print_dedup_error_human,
                print_dedup_error_json,
            )?;
            if !palace_exists(&config) {
                if human {
                    print_dedup_no_palace_human(&config);
                } else {
                    print_no_palace(&config)?;
                }
                std::process::exit(1);
            }
            let app = create_app(
                config,
                human,
                print_dedup_error_human,
                print_dedup_error_json,
            )?;
            let summary = match app
                .dedup(
                    threshold,
                    dry_run,
                    wing.as_deref(),
                    source.as_deref(),
                    5,
                    stats,
                )
                .await
            {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_dedup_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_dedup_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_dedup_human(&summary);
            } else {
                print_json(&summary)?;
            }
        }
        PalaceCommand::Status { human } => {
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
            let app = App::new(config)?;
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
        }
        PalaceCommand::Doctor {
            warm_embedding,
            human,
        } => {
            let config = resolve_config(
                palace,
                hf_endpoint,
                human,
                print_doctor_error_human,
                print_doctor_error_json,
            )?;
            let app = create_app(
                config,
                human,
                print_doctor_error_human,
                print_doctor_error_json,
            )?;
            let summary = match app.doctor(warm_embedding).await {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_doctor_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_doctor_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_doctor_human(&summary);
            } else {
                print_json(&summary)?;
            }
        }
        PalaceCommand::PrepareEmbedding {
            attempts,
            wait_ms,
            human,
        } => {
            let config = resolve_config(
                palace,
                hf_endpoint,
                human,
                print_prepare_embedding_error_human,
                print_prepare_embedding_error_json,
            )?;
            let app = create_app(
                config,
                human,
                print_prepare_embedding_error_human,
                print_prepare_embedding_error_json,
            )?;
            let summary = match app.prepare_embedding(attempts, wait_ms).await {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_prepare_embedding_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_prepare_embedding_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_prepare_embedding_human(&summary);
            } else {
                print_json(&summary)?;
            }
        }
    }

    Ok(())
}

fn resolve_config(
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
    print_human_error: fn(&str),
    print_json_error: fn(&str) -> Result<()>,
) -> Result<AppConfig> {
    let mut config = match AppConfig::resolve(palace) {
        Ok(config) => config,
        Err(err) if human => {
            print_human_error(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_json_error(&err.to_string())?;
            std::process::exit(1);
        }
    };
    apply_cli_overrides(&mut config, hf_endpoint);
    Ok(config)
}

fn create_app(
    config: AppConfig,
    human: bool,
    print_human_error: fn(&str),
    print_json_error: fn(&str) -> Result<()>,
) -> Result<App> {
    match App::new(config) {
        Ok(app) => Ok(app),
        Err(err) if human => {
            print_human_error(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_json_error(&err.to_string())?;
            std::process::exit(1);
        }
    }
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
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

fn print_doctor_human(summary: &DoctorSummary) {
    print!("{}", render_doctor_human(summary));
}

fn print_doctor_error_human(message: &str) {
    println!("\n  Doctor error: {message}");
    println!("  Check the embedding provider and local runtime, then rerun `mempalace-rs doctor`.");
}

fn print_doctor_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Doctor error: {message}"),
        "hint": "Check the embedding provider and local runtime, then rerun `mempalace-rs doctor`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn render_doctor_human(summary: &DoctorSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!("\n{}\n", "=".repeat(55)));
    out.push_str("  MemPalace Doctor\n");
    out.push_str(&format!("{}\n\n", "=".repeat(55)));
    out.push_str(&format!("  Palace:     {}\n", summary.palace_path));
    out.push_str(&format!("  SQLite:     {}\n", summary.sqlite_path));
    out.push_str(&format!("  LanceDB:    {}\n", summary.lance_path));
    out.push_str(&format!("  Provider:   {}\n", summary.provider));
    out.push_str(&format!("  Model:      {}\n", summary.model));
    out.push_str(&format!("  Dimension:  {}\n", summary.dimension));
    if let Some(path) = &summary.cache_dir {
        out.push_str(&format!("  Cache dir:  {path}\n"));
    }
    if let Some(path) = &summary.model_cache_dir {
        out.push_str(&format!("  Model dir:  {path}\n"));
    }
    if let Some(path) = &summary.expected_model_file {
        out.push_str(&format!("  Model file: {path}\n"));
    }
    out.push_str(&format!(
        "  Cache hit:  {}\n",
        if summary.model_cache_present {
            "yes"
        } else {
            "no"
        }
    ));
    out.push_str(&format!(
        "  Model file present: {}\n",
        if summary.expected_model_file_present {
            "yes"
        } else {
            "no"
        }
    ));
    if let Some(path) = &summary.ort_dylib_path {
        out.push_str(&format!("  ORT dylib:  {path}\n"));
    }
    if let Some(endpoint) = &summary.hf_endpoint {
        out.push_str(&format!("  HF endpoint: {endpoint}\n"));
    }
    if !summary.model_cache_present {
        out.push_str("  Cache state: model cache directory not populated yet\n");
    } else if !summary.expected_model_file_present {
        out.push_str("  Cache state: model snapshot exists but onnx/model.onnx is missing\n");
    } else {
        out.push_str("  Cache state: model snapshot looks ready\n");
    }
    if summary.warmup_attempted {
        out.push_str(&format!(
            "  Warmup:     {}\n",
            if summary.warmup_ok { "ok" } else { "failed" }
        ));
        if let Some(error) = &summary.warmup_error {
            out.push_str(&format!("  Warmup err: {error}\n"));
        }
        if !summary.warmup_ok {
            out.push_str("\n  Suggested next step:\n");
            if summary.hf_endpoint.is_none() {
                out.push_str(
                    "    Retry with --hf-endpoint https://hf-mirror.com if the default HuggingFace route is blocked.\n",
                );
            } else {
                out.push_str(
                    "    Retry prepare-embedding after verifying the configured HuggingFace mirror and local network access.\n",
                );
            }
        }
    }
    out.push_str(&format!("\n{}\n\n", "=".repeat(55)));
    out
}

fn print_prepare_embedding_human(summary: &PrepareEmbeddingSummary) {
    print!("{}", render_prepare_embedding_human(summary));
}

fn render_prepare_embedding_human(summary: &PrepareEmbeddingSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!("\n{}\n", "=".repeat(55)));
    out.push_str("  MemPalace Prepare Embedding\n");
    out.push_str(&format!("{}\n\n", "=".repeat(55)));
    out.push_str(&format!("  Palace:    {}\n", summary.palace_path));
    out.push_str(&format!("  Provider:  {}\n", summary.provider));
    out.push_str(&format!("  Model:     {}\n", summary.model));
    out.push_str(&format!("  Attempts:  {}\n", summary.attempts));
    out.push_str(&format!(
        "  Result:    {}\n",
        if summary.success { "ok" } else { "failed" }
    ));
    if let Some(error) = &summary.last_error {
        out.push_str(&format!("  Last err:  {error}\n"));
    }
    out.push_str(&format!(
        "  Warmup:    {}\n",
        if summary.doctor.warmup_ok {
            "ok"
        } else {
            "failed"
        }
    ));
    if let Some(path) = &summary.doctor.model_cache_dir {
        out.push_str(&format!("  Model dir: {path}\n"));
    }
    if let Some(path) = &summary.doctor.expected_model_file {
        out.push_str(&format!("  Model file: {path}\n"));
    }
    out.push_str(&format!(
        "  Model file present: {}\n",
        if summary.doctor.expected_model_file_present {
            "yes"
        } else {
            "no"
        }
    ));
    if !summary.success {
        out.push_str("\n  Suggested next step:\n");
        if summary.doctor.hf_endpoint.is_none() {
            out.push_str(
                "    Retry with --hf-endpoint https://hf-mirror.com if model download cannot reach HuggingFace.\n",
            );
        } else {
            out.push_str(
                "    Verify the configured HuggingFace mirror and rerun prepare-embedding once model download works.\n",
            );
        }
    }
    out.push_str(&format!("\n{}\n\n", "=".repeat(55)));
    out
}

fn print_prepare_embedding_error_human(message: &str) {
    println!("\n  Prepare embedding error: {message}");
    println!(
        "  Check the palace files and embedding runtime, then rerun `mempalace-rs prepare-embedding`."
    );
}

fn print_prepare_embedding_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Prepare embedding error: {message}"),
        "hint": "Check the palace files and embedding runtime, then rerun `mempalace-rs prepare-embedding`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{render_doctor_human, render_prepare_embedding_human};
    use mempalace_rs::model::{DoctorSummary, PrepareEmbeddingSummary};

    fn failed_doctor_summary(hf_endpoint: Option<&str>) -> DoctorSummary {
        DoctorSummary {
            kind: "doctor".to_string(),
            palace_path: "/tmp/palace".to_string(),
            sqlite_path: "/tmp/palace/palace.sqlite3".to_string(),
            lance_path: "/tmp/palace/lance".to_string(),
            version: "0.1.0".to_string(),
            provider: "fastembed".to_string(),
            model: "MultilingualE5Small".to_string(),
            dimension: 384,
            cache_dir: Some("/tmp/cache".to_string()),
            model_cache_dir: Some("/tmp/cache/model".to_string()),
            model_cache_present: false,
            expected_model_file: Some("/tmp/cache/model/onnx/model.onnx".to_string()),
            expected_model_file_present: false,
            hf_endpoint: hf_endpoint.map(ToOwned::to_owned),
            ort_dylib_path: Some(
                "/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib".to_string(),
            ),
            warmup_attempted: true,
            warmup_ok: false,
            warmup_error: Some("Failed to retrieve onnx/model.onnx".to_string()),
        }
    }

    #[test]
    fn doctor_human_failure_suggests_mirror_when_default_endpoint_fails() {
        let output = render_doctor_human(&failed_doctor_summary(None));
        assert!(output.contains("Cache state: model cache directory not populated yet"));
        assert!(output.contains("Warmup:     failed"));
        assert!(output.contains("Suggested next step:"));
        assert!(output.contains("--hf-endpoint https://hf-mirror.com"));
    }

    #[test]
    fn prepare_embedding_human_failure_mentions_configured_mirror_when_present() {
        let doctor = failed_doctor_summary(Some("https://hf-mirror.example"));
        let output = render_prepare_embedding_human(&PrepareEmbeddingSummary {
            kind: "prepare_embedding".to_string(),
            palace_path: "/tmp/palace".to_string(),
            sqlite_path: "/tmp/palace/palace.sqlite3".to_string(),
            lance_path: "/tmp/palace/lance".to_string(),
            version: "0.1.0".to_string(),
            provider: "fastembed".to_string(),
            model: "MultilingualE5Small".to_string(),
            attempts: 1,
            success: false,
            last_error: Some("Failed to retrieve onnx/model.onnx".to_string()),
            doctor,
        });
        assert!(output.contains("Result:    failed"));
        assert!(output.contains("Last err:  Failed to retrieve onnx/model.onnx"));
        assert!(output.contains("Suggested next step:"));
        assert!(output.contains("Verify the configured HuggingFace mirror"));
    }
}
