use std::io::Write;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mempalace_rs::config::AppConfig;
use mempalace_rs::convo::normalize_conversation_file;
use mempalace_rs::hook;
use mempalace_rs::instructions;
use mempalace_rs::mcp;
use mempalace_rs::model::{MineProgressEvent, MineRequest};
use mempalace_rs::onboarding::{
    OnboardingRequest, parse_alias_arg, parse_person_arg, run_onboarding,
};
use mempalace_rs::service::App;
use mempalace_rs::split;
use serde_json::json;

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
    #[command(about = "Run the read-only MCP server on stdio")]
    Mcp {
        #[arg(long)]
        #[arg(help = "Print Python-style MCP setup instructions instead of starting the server")]
        setup: bool,
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

#[derive(Subcommand)]
enum RepairCommand {
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

#[derive(Subcommand)]
enum RegistryCommand {
    #[command(about = "Show a summary of entity_registry.json")]
    Summary {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable registry summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Look up one word in entity_registry.json")]
    Lookup {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Word to look up")]
        word: String,
        #[arg(long, default_value = "")]
        #[arg(help = "Context sentence used for ambiguous-name disambiguation")]
        context: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable lookup output instead of JSON")]
        human: bool,
    },
    #[command(about = "Learn new people/projects from local files into entity_registry.json")]
    Learn {
        #[arg(help = "Project directory to scan and update")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable learn summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Add one person to entity_registry.json")]
    AddPerson {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Person name to add")]
        name: String,
        #[arg(long, default_value = "")]
        #[arg(help = "Relationship or role")]
        relationship: String,
        #[arg(long, default_value = "work")]
        #[arg(help = "Context bucket: work or personal")]
        context: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable write summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Add one project to entity_registry.json")]
    AddProject {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Project name to add")]
        name: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable write summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Add an alias for an existing person in entity_registry.json")]
    AddAlias {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Existing canonical person name")]
        canonical: String,
        #[arg(help = "Alias or nickname to add")]
        alias: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable write summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Extract known people and unknown capitalized candidates from a query")]
    Query {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Query text to inspect")]
        query: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable query summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Research one unknown word into the registry wiki cache")]
    Research {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Word to research via Wikipedia")]
        word: String,
        #[arg(long)]
        #[arg(help = "Mark the researched result as confirmed immediately")]
        auto_confirm: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable research summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Confirm one researched word and promote it into the registry")]
    Confirm {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Word already present in the wiki cache")]
        word: String,
        #[arg(long = "type", default_value = "person")]
        #[arg(help = "Confirmed entity type, usually person")]
        entity_type: String,
        #[arg(long, default_value = "")]
        #[arg(help = "Relationship or role if confirming a person")]
        relationship: String,
        #[arg(long, default_value = "personal")]
        #[arg(help = "Context bucket: work or personal")]
        context: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable confirm summary instead of JSON")]
        human: bool,
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
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_compress_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_compress_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if !palace_exists(&config) {
                if human {
                    print_compress_no_palace_human(&config);
                } else {
                    print_no_palace(&config)?;
                }
                std::process::exit(1);
            }
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_compress_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_compress_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
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
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::WakeUp { wing, human } => {
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_wake_up_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_wake_up_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if !palace_exists(&config) {
                if human {
                    print_wake_up_no_palace_human(&config);
                } else {
                    print_no_palace(&config)?;
                }
                std::process::exit(1);
            }
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_wake_up_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_wake_up_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
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
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Migrate { human } => {
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_migrate_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_migrate_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if human && !palace_exists(&config) {
                print_migrate_no_palace_human(&config);
                return Ok(());
            }
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_migrate_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_migrate_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
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
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Repair { action, human } => {
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_repair_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_repair_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if human && !palace_exists(&config) {
                print_repair_no_palace_human(&config);
                return Ok(());
            }
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_repair_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_repair_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
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
                        println!("{}", serde_json::to_string_pretty(&summary)?);
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
                        println!("{}", serde_json::to_string_pretty(&summary)?);
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
                        println!("{}", serde_json::to_string_pretty(&summary)?);
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
                        println!("{}", serde_json::to_string_pretty(&summary)?);
                    }
                }
            }
        }
        Command::Dedup {
            threshold,
            dry_run,
            stats,
            wing,
            source,
            human,
        } => {
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_dedup_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_dedup_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if !palace_exists(&config) {
                if human {
                    print_dedup_no_palace_human(&config);
                } else {
                    print_no_palace(&config)?;
                }
                std::process::exit(1);
            }
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_dedup_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_dedup_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
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
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Status { human } => {
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_status_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_status_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
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
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Doctor {
            warm_embedding,
            human,
        } => {
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_doctor_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_doctor_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_doctor_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_doctor_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
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
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::PrepareEmbedding {
            attempts,
            wait_ms,
            human,
        } => {
            let mut config = match AppConfig::resolve(palace.as_ref()) {
                Ok(config) => config,
                Err(err) if human => {
                    print_prepare_embedding_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_prepare_embedding_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = match App::new(config) {
                Ok(app) => app,
                Err(err) if human => {
                    print_prepare_embedding_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_prepare_embedding_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
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
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
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
        Command::Registry { action } => match action {
            RegistryCommand::Summary { dir, human } => {
                let mut config = AppConfig::resolve(palace.as_ref())?;
                apply_cli_overrides(&mut config, hf_endpoint.as_deref());
                let app = App::new(config)?;
                let summary = app.registry_summary(&dir)?;
                if human {
                    print_registry_summary_human(&summary);
                } else {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
            RegistryCommand::Lookup {
                dir,
                word,
                context,
                human,
            } => {
                let mut config = AppConfig::resolve(palace.as_ref())?;
                apply_cli_overrides(&mut config, hf_endpoint.as_deref());
                let app = App::new(config)?;
                let summary = app.registry_lookup(&dir, &word, &context)?;
                if human {
                    print_registry_lookup_human(&summary);
                } else {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
            RegistryCommand::Learn { dir, human } => {
                let mut config = AppConfig::resolve(palace.as_ref())?;
                apply_cli_overrides(&mut config, hf_endpoint.as_deref());
                let app = App::new(config)?;
                let summary = app.registry_learn(&dir)?;
                if human {
                    print_registry_learn_human(&summary);
                } else {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
            RegistryCommand::AddPerson {
                dir,
                name,
                relationship,
                context,
                human,
            } => {
                let mut config = AppConfig::resolve(palace.as_ref())?;
                apply_cli_overrides(&mut config, hf_endpoint.as_deref());
                let app = App::new(config)?;
                let summary = app.registry_add_person(&dir, &name, &relationship, &context)?;
                if human {
                    print_registry_write_human(&summary);
                } else {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
            RegistryCommand::AddProject { dir, name, human } => {
                let mut config = AppConfig::resolve(palace.as_ref())?;
                apply_cli_overrides(&mut config, hf_endpoint.as_deref());
                let app = App::new(config)?;
                let summary = app.registry_add_project(&dir, &name)?;
                if human {
                    print_registry_write_human(&summary);
                } else {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
            RegistryCommand::AddAlias {
                dir,
                canonical,
                alias,
                human,
            } => {
                let mut config = AppConfig::resolve(palace.as_ref())?;
                apply_cli_overrides(&mut config, hf_endpoint.as_deref());
                let app = App::new(config)?;
                let summary = app.registry_add_alias(&dir, &canonical, &alias)?;
                if human {
                    print_registry_write_human(&summary);
                } else {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
            RegistryCommand::Query { dir, query, human } => {
                let mut config = AppConfig::resolve(palace.as_ref())?;
                apply_cli_overrides(&mut config, hf_endpoint.as_deref());
                let app = App::new(config)?;
                let summary = app.registry_query(&dir, &query)?;
                if human {
                    print_registry_query_human(&summary);
                } else {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
            RegistryCommand::Research {
                dir,
                word,
                auto_confirm,
                human,
            } => {
                let mut config = AppConfig::resolve(palace.as_ref())?;
                apply_cli_overrides(&mut config, hf_endpoint.as_deref());
                let app = App::new(config)?;
                let summary = app.registry_research(&dir, &word, auto_confirm)?;
                if human {
                    print_registry_research_human(&summary);
                } else {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
            RegistryCommand::Confirm {
                dir,
                word,
                entity_type,
                relationship,
                context,
                human,
            } => {
                let mut config = AppConfig::resolve(palace.as_ref())?;
                apply_cli_overrides(&mut config, hf_endpoint.as_deref());
                let app = App::new(config)?;
                let summary = app.registry_confirm_research(
                    &dir,
                    &word,
                    &entity_type,
                    &relationship,
                    &context,
                )?;
                if human {
                    print_registry_confirm_human(&summary);
                } else {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                }
            }
        },
        Command::Mcp { setup } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if setup {
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
    let base_server_cmd = "mempalace-rs mcp";
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
    if summary.results.is_empty() {
        println!("\n  No results found for: \"{}\"", summary.query);
        return;
    }

    println!("\n{}", "=".repeat(60));
    println!("  Results for: \"{}\"", summary.query);
    if let Some(wing) = &summary.filters.wing {
        println!("  Wing: {wing}");
    }
    if let Some(room) = &summary.filters.room {
        println!("  Room: {room}");
    }
    println!("{}\n", "=".repeat(60));

    for (index, hit) in summary.results.iter().enumerate() {
        let similarity = hit
            .similarity
            .map(|value| value.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!("  [{}] {} / {}", index + 1, hit.wing, hit.room);
        println!("      Source: {}", hit.source_file);
        println!("      Match:  {similarity}");
        println!();
        for line in hit.text.trim().lines() {
            println!("      {line}");
        }
        println!();
        println!("  {}", "─".repeat(56));
    }
    println!();
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

fn print_compress_human(summary: &mempalace_rs::model::CompressSummary) {
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

fn print_compress_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Compress error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs compress`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_wake_up_human(summary: &mempalace_rs::model::WakeUpSummary) {
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

fn print_wake_up_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Wake-up error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs wake-up`.",
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

fn print_doctor_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Doctor error: {message}"),
        "hint": "Check the embedding provider and local runtime, then rerun `mempalace-rs doctor`.",
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

fn print_status_human(
    summary: &mempalace_rs::model::Status,
    taxonomy: &mempalace_rs::model::Taxonomy,
) {
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

fn print_status_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Status error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs status`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_repair_human(summary: &mempalace_rs::model::RepairSummary) {
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

fn print_repair_scan_human(summary: &mempalace_rs::model::RepairScanSummary) {
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

fn print_repair_prune_human(summary: &mempalace_rs::model::RepairPruneSummary) {
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

fn print_repair_rebuild_human(summary: &mempalace_rs::model::RepairRebuildSummary) {
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

fn print_repair_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Repair error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs repair`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_dedup_human(summary: &mempalace_rs::model::DedupSummary) {
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

fn print_dedup_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Dedup error: {message}"),
        "hint": "Check the palace files, then rerun `mempalace-rs dedup`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_registry_summary_human(summary: &mempalace_rs::model::RegistrySummaryResult) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Registry");
    println!("{}\n", "=".repeat(55));
    println!("  Registry: {}", summary.registry_path);
    println!("  Mode: {}", summary.mode);
    println!("  People: {}", summary.people_count);
    println!("  Projects: {}", summary.project_count);
    if !summary.ambiguous_flags.is_empty() {
        println!("  Ambiguous flags: {}", summary.ambiguous_flags.join(", "));
    }
    if !summary.people.is_empty() {
        println!("\n  People:");
        for person in &summary.people {
            println!("    - {person}");
        }
    }
    if !summary.projects.is_empty() {
        println!("\n  Projects:");
        for project in &summary.projects {
            println!("    - {project}");
        }
    }
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_registry_lookup_human(summary: &mempalace_rs::model::RegistryLookupResult) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Registry Lookup");
    println!("{}\n", "=".repeat(55));
    println!("  Registry: {}", summary.registry_path);
    println!("  Word: {}", summary.word);
    println!("  Type: {}", summary.r#type);
    println!("  Name: {}", summary.name);
    println!("  Confidence: {:.2}", summary.confidence);
    println!("  Source: {}", summary.source);
    if !summary.context.is_empty() {
        println!("  Contexts: {}", summary.context.join(", "));
    }
    if let Some(disambiguated_by) = &summary.disambiguated_by {
        println!("  Disambiguated by: {disambiguated_by}");
    }
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_registry_learn_human(summary: &mempalace_rs::model::RegistryLearnResult) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Registry Learn");
    println!("{}\n", "=".repeat(55));
    println!("  Project: {}", summary.project_path);
    println!("  Registry: {}", summary.registry_path);
    println!("  Added people: {}", summary.added_people.len());
    println!("  Added projects: {}", summary.added_projects.len());
    if !summary.added_people.is_empty() {
        println!("\n  New people:");
        for person in &summary.added_people {
            println!("    - {person}");
        }
    }
    if !summary.added_projects.is_empty() {
        println!("\n  New projects:");
        for project in &summary.added_projects {
            println!("    - {project}");
        }
    }
    println!(
        "\n  Totals: {} people, {} projects",
        summary.total_people, summary.total_projects
    );
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_registry_write_human(summary: &mempalace_rs::model::RegistryWriteResult) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Registry Write");
    println!("{}\n", "=".repeat(55));
    println!("  Registry: {}", summary.registry_path);
    println!("  Action: {}", summary.action);
    println!("  Name: {}", summary.name);
    if let Some(canonical) = &summary.canonical {
        println!("  Canonical: {canonical}");
    }
    println!("  Mode: {}", summary.mode);
    println!(
        "  Totals: {} people, {} projects",
        summary.people_count, summary.project_count
    );
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_registry_query_human(summary: &mempalace_rs::model::RegistryQueryResult) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Registry Query");
    println!("{}\n", "=".repeat(55));
    println!("  Registry: {}", summary.registry_path);
    println!("  Query: {}", summary.query);
    println!("  People: {}", summary.people.join(", "));
    println!(
        "  Unknown candidates: {}",
        summary.unknown_candidates.join(", ")
    );
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_registry_research_human(summary: &mempalace_rs::model::RegistryResearchResult) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Registry Research");
    println!("{}\n", "=".repeat(55));
    println!("  Registry: {}", summary.registry_path);
    println!("  Word: {}", summary.word);
    println!("  Inferred type: {}", summary.inferred_type);
    println!("  Confidence: {:.0}%", summary.confidence * 100.0);
    if let Some(title) = &summary.wiki_title {
        println!("  Wiki title: {title}");
    }
    if let Some(note) = &summary.note {
        println!("  Note: {note}");
    }
    println!("  Confirmed: {}", summary.confirmed);
    if let Some(confirmed_type) = &summary.confirmed_type {
        println!("  Confirmed type: {confirmed_type}");
    }
    if let Some(wiki_summary) = &summary.wiki_summary {
        println!("  Summary: {wiki_summary}");
    }
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_registry_confirm_human(summary: &mempalace_rs::model::RegistryConfirmResult) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Registry Confirm");
    println!("{}\n", "=".repeat(55));
    println!("  Registry: {}", summary.registry_path);
    println!("  Word: {}", summary.word);
    println!("  Type: {}", summary.entity_type);
    println!("  Relationship: {}", summary.relationship);
    println!("  Context: {}", summary.context);
    println!(
        "  Totals: {} people, {} projects, {} wiki cache entries",
        summary.total_people, summary.total_projects, summary.wiki_cache_entries
    );
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_migrate_human(summary: &mempalace_rs::model::MigrateSummary) {
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

fn print_migrate_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Migrate error: {message}"),
        "hint": "Check the palace SQLite file, then rerun `mempalace-rs migrate`.",
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
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

fn print_doctor_human(summary: &mempalace_rs::model::DoctorSummary) {
    print!("{}", render_doctor_human(summary));
}

fn print_doctor_error_human(message: &str) {
    println!("\n  Doctor error: {message}");
    println!("  Check the embedding provider and local runtime, then rerun `mempalace-rs doctor`.");
}

fn render_doctor_human(summary: &mempalace_rs::model::DoctorSummary) -> String {
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

fn print_prepare_embedding_human(summary: &mempalace_rs::model::PrepareEmbeddingSummary) {
    print!("{}", render_prepare_embedding_human(summary));
}

fn render_prepare_embedding_human(
    summary: &mempalace_rs::model::PrepareEmbeddingSummary,
) -> String {
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

fn print_prepare_embedding_error_json(message: &str) -> anyhow::Result<()> {
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
