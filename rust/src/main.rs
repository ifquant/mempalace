use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mempalace_rs::config::AppConfig;
use mempalace_rs::mcp;
use mempalace_rs::service::App;

#[derive(Parser)]
#[command(name = "mempalace-rs")]
#[command(about = "Rust rewrite of MemPalace")]
struct Cli {
    #[arg(long)]
    palace: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        dir: PathBuf,
    },
    Mine {
        dir: PathBuf,
        #[arg(long)]
        wing: Option<String>,
        #[arg(long, default_value_t = 0)]
        limit: usize,
        #[arg(long)]
        no_gitignore: bool,
        #[arg(long = "include-ignored")]
        include_ignored: Vec<String>,
    },
    Search {
        query: String,
        #[arg(long)]
        wing: Option<String>,
        #[arg(long)]
        room: Option<String>,
        #[arg(long, default_value_t = 5)]
        results: usize,
    },
    Status,
    Doctor {
        #[arg(long)]
        warm_embedding: bool,
    },
    PrepareEmbedding {
        #[arg(long, default_value_t = 3)]
        attempts: usize,
        #[arg(long, default_value_t = 1000)]
        wait_ms: u64,
    },
    Mcp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli { palace, command } = Cli::parse();

    match command {
        Command::Init { dir } => {
            let palace_path = palace.as_ref().unwrap_or(&dir);
            let config = AppConfig::resolve(Some(palace_path))?;
            let app = App::new(config)?;
            let summary = app.init().await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Mine {
            dir,
            wing,
            limit,
            no_gitignore,
            include_ignored,
        } => {
            let config = AppConfig::resolve(palace.as_ref())?;
            let app = App::new(config)?;
            let summary = app
                .mine_project(
                    &dir,
                    wing.as_deref(),
                    limit,
                    !no_gitignore,
                    &include_ignored,
                )
                .await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Search {
            query,
            wing,
            room,
            results,
        } => {
            let config = AppConfig::resolve(palace.as_ref())?;
            let app = App::new(config)?;
            let summary = app
                .search(&query, wing.as_deref(), room.as_deref(), results)
                .await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Status => {
            let config = AppConfig::resolve(palace.as_ref())?;
            let app = App::new(config)?;
            let summary = app.status().await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Doctor { warm_embedding } => {
            let config = AppConfig::resolve(palace.as_ref())?;
            let app = App::new(config)?;
            let summary = app.doctor(warm_embedding).await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::PrepareEmbedding { attempts, wait_ms } => {
            let config = AppConfig::resolve(palace.as_ref())?;
            let app = App::new(config)?;
            let summary = app.prepare_embedding(attempts, wait_ms).await?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Command::Mcp => {
            let config = AppConfig::resolve(palace.as_ref())?;
            mcp::run_stdio(config).await?;
        }
    }

    Ok(())
}
