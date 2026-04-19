use std::path::{Path, PathBuf};

use anyhow::Result;
use mempalace_rs::model::{MineProgressEvent, MineRequest, SearchResults};
use mempalace_rs::searcher::render_search_human;
use serde_json::json;

use crate::cli_support::{palace_exists, print_no_palace};
use crate::project_cli_support::{create_app, print_json, resolve_config};

#[allow(clippy::too_many_arguments)]
pub async fn handle_mine(
    dir: &Path,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
    mode: String,
    wing: Option<String>,
    limit: usize,
    dry_run: bool,
    no_gitignore: bool,
    include_ignored: Vec<String>,
    agent: String,
    extract: String,
    progress: bool,
    human: bool,
) -> Result<()> {
    let config = resolve_config(
        palace,
        hf_endpoint,
        human,
        print_mine_error_human,
        print_mine_error_json,
    )?;
    let app = create_app(config, human, print_mine_error_human, print_mine_error_json)?;
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
        app.mine_project_with_progress(dir, &request, |event| match event {
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
        app.mine_project(dir, &request).await
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
        print_json(&summary)?;
    }
    Ok(())
}

pub async fn handle_search(
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
    query: String,
    wing: Option<String>,
    room: Option<String>,
    results: usize,
    human: bool,
) -> Result<()> {
    let config = resolve_config(
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
    let app = create_app(
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
        print_json(&summary)?;
    }
    Ok(())
}

fn print_search_human(summary: &SearchResults) {
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
    print_json(&payload)
}

fn print_mine_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Mine error: {message}"),
        "hint": "Check the embedding provider, project path, and palace files, then rerun `mempalace-rs mine <dir>`.",
    });
    print_json(&payload)
}

fn print_mine_error_human(message: &str) {
    println!("\n  Mine error: {message}");
    println!(
        "  Check the embedding provider and project path, then rerun `mempalace-rs mine <dir>`."
    );
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
