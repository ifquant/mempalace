use anyhow::Result;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::{Status, Taxonomy};
use serde_json::json;

use crate::cli_support::{palace_exists, print_no_palace};
use crate::palace_cli_read_support::{create_read_app, print_read_json, resolve_read_config};

pub async fn handle_status(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
) -> Result<()> {
    let config = resolve_read_config(
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
    let app = create_read_app(
        config,
        human,
        print_status_error_human,
        print_status_error_json,
    )?;
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
        print_read_json(&summary)?;
    }
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
