use std::path::PathBuf;

use anyhow::Result;
use mempalace_rs::searcher::render_search_human;
use serde_json::json;

use crate::cli_support::{palace_exists, print_no_palace};
use crate::project_cli_mining_support::{
    create_mining_app, print_mining_json, resolve_mining_config,
};

pub async fn handle_search(
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
    query: String,
    wing: Option<String>,
    room: Option<String>,
    results: usize,
    human: bool,
) -> Result<()> {
    let config = resolve_mining_config(
        palace,
        hf_endpoint,
        human,
        print_search_error_human,
        print_search_error_json,
    )?;
    if !palace_exists(&config) {
        if human {
            print_search_no_palace_human(&config);
        } else {
            print_no_palace(&config)?;
        }
        std::process::exit(1);
    }
    let app = create_mining_app(
        config,
        human,
        print_search_error_human,
        print_search_error_json,
    )?;
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
        print_mining_json(&summary)?;
    }
    Ok(())
}

fn print_search_human(summary: &mempalace_rs::model::SearchResults) {
    print!("{}", render_search_human(summary));
}

fn print_search_no_palace_human(config: &mempalace_rs::config::AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
    println!("  Run: mempalace init <dir> then mempalace mine <dir>");
}

fn print_search_error_human(message: &str) {
    println!("\n  Search error: {message}");
}

fn print_search_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Search error: {message}"),
        "hint": "Check the embedding provider, palace files, or query inputs, then rerun `mempalace-rs search <query>`.",
    });
    print_mining_json(&payload)
}
