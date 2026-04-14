use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use mempalace_rs::config::AppConfig;
use mempalace_rs::mcp;
use mempalace_rs::model::{MineProgressEvent, MineRequest};
use mempalace_rs::service::App;
use serde_json::json;

#[derive(Parser)]
#[command(name = "mempalace-rs")]
#[command(
    about = "MemPalace — Give your AI a memory. No API key required.",
    long_about = "MemPalace — Give your AI a memory. No API key required.\n\nCurrent Rust phase supports local-first project mining, search, migration, repair diagnostics, and read-only MCP tools.\n\nExamples:\n  mempalace-rs init ~/projects/my_app\n  mempalace-rs mine ~/projects/my_app\n  mempalace-rs search \"why did we switch to GraphQL\"\n  mempalace-rs status\n  mempalace-rs migrate\n  mempalace-rs repair"
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
    #[command(about = "Upgrade palace SQLite metadata to the current schema version")]
    Migrate,
    #[command(about = "Run non-destructive palace diagnostics")]
    Repair,
    #[command(about = "Show what has been filed in the palace")]
    Status,
    #[command(about = "Inspect embedding runtime health and cache state")]
    Doctor {
        #[arg(long)]
        #[arg(help = "Warm the embedding model during the doctor run")]
        warm_embedding: bool,
    },
    #[command(about = "Prepare the local embedding runtime and model cache")]
    PrepareEmbedding {
        #[arg(long, default_value_t = 3)]
        #[arg(help = "How many warm-up attempts to make")]
        attempts: usize,
        #[arg(long, default_value_t = 1000)]
        #[arg(help = "Milliseconds to wait between attempts")]
        wait_ms: u64,
    },
    #[command(about = "Run the read-only MCP server on stdio")]
    Mcp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli {
        palace,
        hf_endpoint,
        command,
    } = Cli::parse();

    match command {
        Command::Init { dir } => {
            let palace_path = palace.as_ref().unwrap_or(&dir);
            let mut config = AppConfig::resolve(Some(palace_path))?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
            let summary = app.init().await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
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
        } => {
            if mode != "projects" {
                print_unsupported_mine_mode(&mode, &extract, &dir)?;
                std::process::exit(2);
            }
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
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
                    MineProgressEvent::Filed {
                        index,
                        total,
                        file_name,
                        drawers,
                    } => {
                        eprintln!("  [ {index:>4}/{total}] {file_name:<50} +{drawers}");
                    }
                })
                .await?
            } else {
                app.mine_project(&dir, &request).await?
            };
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Search {
            query,
            wing,
            room,
            results,
            human,
        } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if !palace_exists(&config) {
                if human {
                    print_search_no_palace_human(&config);
                } else {
                    print_no_palace(&config)?;
                }
                std::process::exit(1);
            }
            let app = App::new(config)?;
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
        Command::Migrate => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
            let summary = app.migrate().await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Repair => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
            let summary = app.repair().await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Status => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if !palace_exists(&config) {
                print_no_palace(&config)?;
                return Ok(());
            }
            let app = App::new(config)?;
            let summary = app.status().await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Doctor { warm_embedding } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
            let summary = app.doctor(warm_embedding).await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::PrepareEmbedding { attempts, wait_ms } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
            let summary = app.prepare_embedding(attempts, wait_ms).await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Mcp => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            mcp::run_stdio(config).await?;
        }
    }

    Ok(())
}

fn apply_cli_overrides(config: &mut AppConfig, hf_endpoint: Option<&str>) {
    if let Some(endpoint) = hf_endpoint {
        config.embedding.hf_endpoint = Some(endpoint.to_string());
    }
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
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_unsupported_mine_mode(mode: &str, extract: &str, dir: &Path) -> anyhow::Result<()> {
    let payload = json!({
        "error": "Unsupported mine mode",
        "hint": "Rust currently supports only `mempalace mine <dir>` project ingest. Conversation and general extraction modes are not implemented yet.",
        "mode": mode,
        "extract": extract,
        "project_path": dir.display().to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}
