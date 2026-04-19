use std::io::Write;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mempalace_rs::config::AppConfig;
use mempalace_rs::hook;
use mempalace_rs::instructions;
use mempalace_rs::mcp;
use mempalace_rs::model::{MineProgressEvent, MineRequest};
use mempalace_rs::normalize::normalize_conversation_file;
use mempalace_rs::onboarding::{
    OnboardingRequest, parse_alias_arg, parse_person_arg, run_onboarding,
};
use mempalace_rs::service::App;
use mempalace_rs::split;
use serde_json::json;

mod palace_cli;
mod registry_cli;

use palace_cli::{PalaceCommand, RepairCommand, handle_palace_command};
use registry_cli::{RegistryCommand, handle_registry_command};

#[derive(Parser)]
#[command(name = "mempalace-rs")]
#[command(
    about = "MemPalace — Give your AI a memory. No API key required.",
    long_about = "MemPalace — Give your AI a memory. No API key required.\n\nCurrent Rust phase supports local-first mining, search, AAAK compression, wake-up context, migration, repair diagnostics, and MCP tools.\n\nExamples:\n  mempalace-rs init ~/projects/my_app\n  mempalace-rs mine ~/projects/my_app\n  mempalace-rs search \"why did we switch to GraphQL\"\n  mempalace-rs compress --wing my_app\n  mempalace-rs wake-up --wing my_app\n  mempalace-rs status"
)]
struct Cli {
    #[arg(long)]
    #[arg(
        help = "Where the palace lives (default: ~/.mempalace-rs/palace or MEMPALACE_RS_PALACE_PATH)"
    )]
    palace: Option<PathBuf>,
    #[arg(long)]
    #[arg(help = "Override the HuggingFace endpoint used by fastembed model downloads")]
    hf_endpoint: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Set up a palace directory for a project")]
    Init {
        #[arg(help = "Project directory to set up")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(
            help = "Auto-accept detected bootstrap files (Rust init is already non-interactive)"
        )]
        yes: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable init summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Guide first-run registry and AAAK bootstrap for a project")]
    Onboarding {
        #[arg(help = "Project directory to seed")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(help = "Usage mode: work, personal, or combo")]
        mode: Option<String>,
        #[arg(long = "person")]
        #[arg(help = "Seed person as name,relationship,context; repeat as needed")]
        people: Vec<String>,
        #[arg(long = "project")]
        #[arg(help = "Seed one project name; repeat as needed")]
        projects: Vec<String>,
        #[arg(long = "alias")]
        #[arg(help = "Seed alias mapping as alias=canonical; repeat as needed")]
        aliases: Vec<String>,
        #[arg(long)]
        #[arg(help = "Comma-separated wing list; defaults follow the selected mode")]
        wings: Option<String>,
        #[arg(long)]
        #[arg(help = "Scan local files for additional names before writing the registry")]
        scan: bool,
        #[arg(long)]
        #[arg(help = "Auto-accept detected names during scan")]
        auto_accept_detected: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable onboarding summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Mine project files into the palace")]
    Mine {
        #[arg(help = "Directory to mine")]
        dir: PathBuf,
        #[arg(long, default_value = "projects")]
        #[arg(help = "Ingest mode: 'projects' for code/docs (default), 'convos' for chat exports")]
        mode: String,
        #[arg(long)]
        #[arg(help = "Wing name (default: mempalace.yaml wing or directory name)")]
        wing: Option<String>,
        #[arg(long, default_value_t = 0)]
        #[arg(help = "Max files to process (0 = all)")]
        limit: usize,
        #[arg(long)]
        #[arg(help = "Preview what would be mined without writing drawers to the palace")]
        dry_run: bool,
        #[arg(long)]
        #[arg(help = "Do not respect .gitignore files when scanning project files")]
        no_gitignore: bool,
        #[arg(long = "include-ignored")]
        #[arg(
            help = "Always scan these project-relative paths even if ignored; repeat or pass comma-separated paths"
        )]
        include_ignored: Vec<String>,
        #[arg(long, default_value = "mempalace")]
        #[arg(help = "Your name — recorded on every drawer (default: mempalace)")]
        agent: String,
        #[arg(long, default_value = "exchange")]
        #[arg(
            help = "Extraction strategy for convos mode: 'exchange' (default) or 'general' (5 memory types)"
        )]
        extract: String,
        #[arg(long)]
        #[arg(
            help = "Print Python-style per-file mining progress to stderr while keeping JSON on stdout"
        )]
        progress: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable mine summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Find anything, exact words")]
    Search {
        #[arg(help = "What to search for")]
        query: String,
        #[arg(long)]
        #[arg(help = "Limit to one project/wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Limit to one room")]
        room: Option<String>,
        #[arg(long, default_value_t = 5)]
        #[arg(help = "Number of results")]
        results: usize,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable search output instead of JSON")]
        human: bool,
    },
    #[command(about = "Split concatenated transcript mega-files into per-session files")]
    Split {
        #[arg(help = "Directory containing transcript files")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(help = "Write split files here (default: same directory as source files)")]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = 2)]
        #[arg(help = "Only split files containing at least N sessions")]
        min_sessions: usize,
        #[arg(long)]
        #[arg(help = "Show what would be split without writing files")]
        dry_run: bool,
    },
    #[command(about = "Normalize one chat export into MemPalace transcript format")]
    Normalize {
        #[arg(help = "Chat export or transcript file to normalize")]
        file: PathBuf,
        #[arg(long)]
        #[arg(help = "Print human-readable preview instead of JSON")]
        human: bool,
    },
    #[command(about = "Compress drawers into AAAK summaries")]
    Compress {
        #[arg(long)]
        #[arg(help = "Limit compression to one project/wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Preview AAAK summaries without storing them")]
        dry_run: bool,
        #[arg(long)]
        #[arg(help = "Print human-readable compression summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Show L0 + L1 wake-up context")]
    WakeUp {
        #[arg(long)]
        #[arg(help = "Show wake-up context for one project/wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Print human-readable wake-up context instead of JSON")]
        human: bool,
    },
    #[command(about = "Recall stored drawers by wing/room without semantic search")]
    Recall {
        #[arg(long)]
        #[arg(help = "Limit recall to one project/wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Limit recall to one room")]
        room: Option<String>,
        #[arg(long, default_value_t = 10)]
        #[arg(help = "Maximum number of drawers to return")]
        results: usize,
        #[arg(long)]
        #[arg(help = "Print human-readable recall output instead of JSON")]
        human: bool,
    },
    #[command(about = "Show Layer 0-3 stack status")]
    LayersStatus {
        #[arg(long)]
        #[arg(help = "Print human-readable layer status instead of JSON")]
        human: bool,
    },
    #[command(about = "Upgrade palace SQLite metadata to the current schema version")]
    Migrate {
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable migration summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Run repair diagnostics or repair subcommands")]
    Repair {
        #[command(subcommand)]
        action: Option<RepairCommand>,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable repair diagnostics instead of JSON")]
        human: bool,
    },
    #[command(about = "Deduplicate near-identical drawers")]
    Dedup {
        #[arg(long, default_value_t = 0.15)]
        #[arg(help = "Cosine distance threshold (lower = stricter)")]
        threshold: f64,
        #[arg(long)]
        #[arg(help = "Preview without deleting")]
        dry_run: bool,
        #[arg(long)]
        #[arg(help = "Show stats only")]
        stats: bool,
        #[arg(long)]
        #[arg(help = "Scope dedup to one wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Filter by source file pattern")]
        source: Option<String>,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable dedup summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Show what has been filed in the palace")]
    Status {
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable palace status instead of JSON")]
        human: bool,
    },
    #[command(about = "Inspect embedding runtime health and cache state")]
    Doctor {
        #[arg(long)]
        #[arg(help = "Warm the embedding model during the doctor run")]
        warm_embedding: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable doctor output instead of JSON")]
        human: bool,
    },
    #[command(about = "Prepare the local embedding runtime and model cache")]
    PrepareEmbedding {
        #[arg(long, default_value_t = 3)]
        #[arg(help = "How many warm-up attempts to make")]
        attempts: usize,
        #[arg(long, default_value_t = 1000)]
        #[arg(help = "Milliseconds to wait between attempts")]
        wait_ms: u64,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable prepare summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Run hook logic (reads JSON from stdin, outputs JSON to stdout)")]
    Hook {
        #[command(subcommand)]
        action: HookCommand,
    },
    #[command(about = "Output skill instructions to stdout")]
    Instructions {
        #[arg(help = "Instruction set name")]
        name: String,
    },
    #[command(about = "Inspect and update the project-local entity registry")]
    Registry {
        #[command(subcommand)]
        action: RegistryCommand,
    },
    #[command(about = "Show MCP setup help or run the read-only MCP server")]
    Mcp {
        #[arg(long)]
        #[arg(help = "Print Python-style MCP setup instructions")]
        setup: bool,
        #[arg(long)]
        #[arg(help = "Run the MCP server on stdio instead of printing setup help")]
        serve: bool,
    },
}

#[derive(Subcommand)]
enum HookCommand {
    #[command(about = "Execute a hook")]
    Run {
        #[arg(long, help = "Hook name to run")]
        hook: String,
        #[arg(long, help = "Harness type")]
        harness: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli {
        palace,
        hf_endpoint,
        command,
    } = Cli::parse();

    match command {
        Command::Init { dir, yes: _, human } => {
            let palace_path = palace.as_ref().unwrap_or(&dir);
            let mut config = match AppConfig::resolve(Some(palace_path)) {
                Ok(config) => config,
                Err(err) if human => {
                    print_init_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_init_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_init_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_init_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            let summary = match app.init_project(&dir).await {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_init_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_init_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_init_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Onboarding {
            dir,
            mode,
            people,
            projects,
            aliases,
            wings,
            scan,
            auto_accept_detected,
            human,
        } => {
            let mut request = OnboardingRequest {
                mode,
                people: Vec::new(),
                projects,
                aliases: std::collections::BTreeMap::new(),
                wings: wings
                    .unwrap_or_default()
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string())
                    .collect(),
                scan: if scan { Some(true) } else { None },
                auto_accept_detected,
            };

            for value in people {
                match parse_person_arg(&value) {
                    Ok(person) => request.people.push(person),
                    Err(err) if human => {
                        print_onboarding_error_human(&err.to_string());
                        std::process::exit(1);
                    }
                    Err(err) => {
                        print_onboarding_error_json(&err.to_string())?;
                        std::process::exit(1);
                    }
                }
            }

            for value in aliases {
                match parse_alias_arg(&value) {
                    Ok((alias, canonical)) => {
                        request.aliases.insert(alias, canonical);
                    }
                    Err(err) if human => {
                        print_onboarding_error_human(&err.to_string());
                        std::process::exit(1);
                    }
                    Err(err) => {
                        print_onboarding_error_json(&err.to_string())?;
                        std::process::exit(1);
                    }
                }
            }

            let summary = match run_onboarding(&dir, request) {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_onboarding_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_onboarding_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_onboarding_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Mine {
            dir,
            mode,
            wing,
            limit,
            dry_run,
            no_gitignore,
            include_ignored,
            agent,
            extract,
            progress,
            human,
        } => {
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_mine_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_mine_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_mine_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_mine_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            let request = MineRequest {
                wing,
                mode,
                agent,
                limit,
                dry_run,
                respect_gitignore: !no_gitignore,
                include_ignored,
                extract,
            };
            let summary = if progress {
                app.mine_project_with_progress(&dir, &request, |event| match event {
                    MineProgressEvent::DryRun {
                        file_name,
                        room,
                        drawers,
                    } => {
                        eprintln!("    [DRY RUN] {file_name} -> room:{room} ({drawers} drawers)");
                    }
                    MineProgressEvent::DryRunSummary {
                        file_name,
                        summary,
                        drawers,
                    } => {
                        eprintln!("    [DRY RUN] {file_name} -> {drawers} memories ({summary})");
                    }
                    MineProgressEvent::Filed {
                        index,
                        total,
                        file_name,
                        drawers,
                    } => {
                        eprintln!("  [ {index:>4}/{total}] {file_name:<50} +{drawers}");
                    }
                })
                .await
            } else {
                app.mine_project(&dir, &request).await
            };
            let summary = match summary {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_mine_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_mine_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_mine_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Search {
            query,
            wing,
            room,
            results,
            human,
        } => {
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_search_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_search_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if !palace_exists(&config) {
                if human {
                    print_search_no_palace_human(&config);
                } else {
                    print_no_palace(&config)?;
                }
                std::process::exit(1);
            }
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_search_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_search_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            let summary = match app
                .search(&query, wing.as_deref(), room.as_deref(), results)
                .await
            {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_search_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_search_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_search_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Split {
            dir,
            output_dir,
            min_sessions,
            dry_run,
        } => {
            let summary =
                split::split_directory(&dir, output_dir.as_deref(), min_sessions, dry_run)?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Normalize { file, human } => {
            let raw = std::fs::read_to_string(&file)?;
            let normalized = normalize_conversation_file(&file)?;
            let Some(normalized) = normalized else {
                if human {
                    print_normalize_error_human("Unsupported or unreadable conversation file.");
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "error": "Normalize error: Unsupported or unreadable conversation file."
                        }))?
                    );
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
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Compress {
            wing,
            dry_run,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::Compress {
                    wing,
                    dry_run,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::WakeUp { wing, human } => {
            handle_palace_command(
                PalaceCommand::WakeUp { wing, human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Recall {
            wing,
            room,
            results,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::Recall {
                    wing,
                    room,
                    results,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::LayersStatus { human } => {
            handle_palace_command(
                PalaceCommand::LayersStatus { human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Migrate { human } => {
            handle_palace_command(
                PalaceCommand::Migrate { human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Repair { action, human } => {
            handle_palace_command(
                PalaceCommand::Repair { action, human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Dedup {
            threshold,
            dry_run,
            stats,
            wing,
            source,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::Dedup {
                    threshold,
                    dry_run,
                    stats,
                    wing,
                    source,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Status { human } => {
            handle_palace_command(
                PalaceCommand::Status { human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Doctor {
            warm_embedding,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::Doctor {
                    warm_embedding,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::PrepareEmbedding {
            attempts,
            wait_ms,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::PrepareEmbedding {
                    attempts,
                    wait_ms,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Hook { action } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            match action {
                HookCommand::Run {
                    hook: hook_name,
                    harness,
                } => {
                    let output = hook::run_hook(&hook_name, &harness, &config)?;
                    writeln!(
                        std::io::stdout(),
                        "{}",
                        serde_json::to_string_pretty(&output)?
                    )?;
                }
            }
        }
        Command::Instructions { name } => {
            let text = instructions::render(&name)?;
            print!("{text}");
        }
        Command::Registry { action } => {
            handle_registry_command(action, palace.as_ref(), hf_endpoint.as_deref())?;
        }
        Command::Mcp { setup, serve } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if setup || !serve {
                print_mcp_setup(&config);
            } else {
                mcp::run_stdio(config).await?;
            }
        }
    }

    Ok(())
}

fn apply_cli_overrides(config: &mut AppConfig, hf_endpoint: Option<&str>) {
    if let Some(endpoint) = hf_endpoint {
        config.embedding.hf_endpoint = Some(endpoint.to_string());
    }
}

fn print_mcp_setup(config: &AppConfig) {
    let base_server_cmd = "mempalace-rs mcp --serve";
    let current_server_cmd = format!(
        "{base_server_cmd} --palace {}",
        shell_quote(&config.palace_path.display().to_string())
    );

    println!("MemPalace MCP quick setup:");
    println!("  claude mcp add mempalace -- {current_server_cmd}");
    println!("\nRun the server directly:");
    println!("  {current_server_cmd}");
    println!("\nOptional custom palace:");
    println!("  claude mcp add mempalace -- {base_server_cmd} --palace /path/to/palace");
    println!("  {base_server_cmd} --palace /path/to/palace");
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '-' | '_' | ':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn palace_exists(config: &AppConfig) -> bool {
    config.sqlite_path().exists() || config.lance_path().exists()
}

fn print_no_palace(config: &AppConfig) -> anyhow::Result<()> {
    let payload = json!({
        "error": "No palace found",
        "hint": "Run: mempalace init <dir> && mempalace mine <dir>",
        "palace_path": config.palace_path.display().to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_search_human(summary: &mempalace_rs::model::SearchResults) {
    print!("{}", mempalace_rs::searcher::render_search_human(summary));
}

fn print_normalize_human(summary: &serde_json::Value) {
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

fn print_search_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
    println!("  Run: mempalace init <dir> then mempalace mine <dir>");
}

fn print_search_error_human(message: &str) {
    println!("\n  Search error: {message}");
}

fn print_search_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Search error: {message}"),
        "hint": "Check the embedding provider, palace files, or query inputs, then rerun `mempalace-rs search <query>`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_mine_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Mine error: {message}"),
        "hint": "Check the embedding provider, project path, and palace files, then rerun `mempalace-rs mine <dir>`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_mine_error_human(message: &str) {
    println!("\n  Mine error: {message}");
    println!(
        "  Check the embedding provider and project path, then rerun `mempalace-rs mine <dir>`."
    );
}

fn print_init_human(summary: &mempalace_rs::model::InitSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Init");
    println!("{}\n", "=".repeat(55));
    println!("  Project: {}", summary.project_path);
    println!("  Wing:    {}", summary.wing);
    println!("  Palace:  {}", summary.palace_path);
    println!("  SQLite:  {}", summary.sqlite_path);
    println!("  LanceDB: {}", summary.lance_path);
    println!("  Schema:  {}", summary.schema_version);
    if !summary.configured_rooms.is_empty() {
        println!("  Rooms:   {}", summary.configured_rooms.join(", "));
    }
    if let Some(config_path) = &summary.config_path {
        println!(
            "  Config:  {}{}",
            config_path,
            if summary.config_written {
                " (written)"
            } else {
                " (kept)"
            }
        );
    }
    if let Some(entities_path) = &summary.entities_path {
        println!(
            "  Entities: {}{}",
            entities_path,
            if summary.entities_written {
                " (written)"
            } else {
                " (kept)"
            }
        );
    }
    if let Some(entity_registry_path) = &summary.entity_registry_path {
        println!(
            "  Registry: {}{}",
            entity_registry_path,
            if summary.entity_registry_written {
                " (written)"
            } else {
                " (kept)"
            }
        );
    }
    if let Some(aaak_entities_path) = &summary.aaak_entities_path {
        println!(
            "  AAAK:    {}{}",
            aaak_entities_path,
            if summary.aaak_entities_written {
                " (written)"
            } else {
                " (kept)"
            }
        );
    }
    if let Some(critical_facts_path) = &summary.critical_facts_path {
        println!(
            "  Facts:   {}{}",
            critical_facts_path,
            if summary.critical_facts_written {
                " (written)"
            } else {
                " (kept)"
            }
        );
    }
    if !summary.detected_people.is_empty() {
        println!("  People:  {}", summary.detected_people.join(", "));
    }
    if !summary.detected_projects.is_empty() {
        println!("  Projects: {}", summary.detected_projects.join(", "));
    }
    println!("\n  Palace initialized.");
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_init_error_human(message: &str) {
    println!("\n  Init error: {message}");
    println!("  Check the palace path and SQLite file, then rerun `mempalace-rs init <dir>`.");
}

fn print_init_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Init error: {message}"),
        "hint": "Check the palace path and SQLite file, then rerun `mempalace-rs init <dir>`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_onboarding_human(summary: &mempalace_rs::model::OnboardingSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Onboarding");
    println!("{}\n", "=".repeat(55));
    println!("  Project: {}", summary.project_path);
    println!("  Mode:    {}", summary.mode);
    println!("  Wing:    {}", summary.wing);
    println!("  Wings:   {}", summary.wings.join(", "));
    println!("  People:  {}", summary.people.len());
    println!("  Projects: {}", summary.projects.len());
    if !summary.aliases.is_empty() {
        println!("  Aliases: {}", summary.aliases.len());
    }
    if !summary.auto_detected_people.is_empty() {
        println!(
            "  Auto-detected people: {}",
            summary.auto_detected_people.join(", ")
        );
    }
    if !summary.auto_detected_projects.is_empty() {
        println!(
            "  Auto-detected projects: {}",
            summary.auto_detected_projects.join(", ")
        );
    }
    if !summary.ambiguous_flags.is_empty() {
        println!("  Ambiguous names: {}", summary.ambiguous_flags.join(", "));
    }
    if let Some(config_path) = &summary.config_path {
        println!(
            "  Config:  {}{}",
            config_path,
            if summary.config_written {
                " (written)"
            } else {
                " (kept)"
            }
        );
    }
    if let Some(entities_path) = &summary.entities_path {
        println!(
            "  Entities: {}{}",
            entities_path,
            if summary.entities_written {
                " (written)"
            } else {
                " (kept)"
            }
        );
    }
    println!(
        "  Registry: {}{}",
        summary.entity_registry_path,
        if summary.entity_registry_written {
            " (written)"
        } else {
            " (kept)"
        }
    );
    println!(
        "  AAAK:    {}{}",
        summary.aaak_entities_path,
        if summary.aaak_entities_written {
            " (written)"
        } else {
            " (kept)"
        }
    );
    println!(
        "  Facts:   {}{}",
        summary.critical_facts_path,
        if summary.critical_facts_written {
            " (written)"
        } else {
            " (kept)"
        }
    );
    println!("\n  Your local world bootstrap is ready.");
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_onboarding_error_human(message: &str) {
    println!("\n  Onboarding error: {message}");
    println!(
        "  Check the project path and onboarding arguments, then rerun `mempalace-rs onboarding <dir>`."
    );
}

fn print_onboarding_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Onboarding error: {message}"),
        "hint": "Check the project path and onboarding arguments, then rerun `mempalace-rs onboarding <dir>`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_mine_human(summary: &mempalace_rs::model::MineSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Mine");
    println!("{}\n", "=".repeat(55));
    println!("  Mode:     {}", summary.mode);
    println!("  Extract:  {}", summary.extract);
    println!("  Wing:     {}", summary.wing);
    println!("  Rooms:    {}", summary.configured_rooms.join(", "));
    println!("  Files:    {}", summary.files_planned);
    println!("  Palace:   {}", summary.palace_path);
    println!("  Project:  {}", summary.project_path);
    println!();
    println!("  Files processed: {}", summary.files_processed);
    println!("  Files skipped:   {}", summary.files_skipped);
    if summary.dry_run {
        println!("  Drawers previewed: {}", summary.drawers_added);
        println!("  Run mode:        DRY RUN");
        println!("  Persistence:     preview only, no drawers were written");
    } else {
        println!("  Drawers filed:   {}", summary.drawers_added);
    }
    if summary.files_planned == 0 {
        println!("\n  No matching files found.");
        println!("  Check your project path, ignore rules, and supported file types.");
    }
    if !summary.room_counts.is_empty() {
        println!("\n  Rooms filed:");
        for (room, count) in &summary.room_counts {
            println!("    - {room}: {count} files");
        }
    }
    println!("\n  {}", summary.next_hint);
    println!("\n{}", "=".repeat(55));
    println!();
}
