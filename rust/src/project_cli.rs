use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::{InitSummary, MineProgressEvent, MineRequest, OnboardingSummary};
use mempalace_rs::normalize::normalize_conversation_file;
use mempalace_rs::onboarding::{
    OnboardingRequest, parse_alias_arg, parse_person_arg, run_onboarding,
};
use mempalace_rs::searcher::render_search_human;
use mempalace_rs::service::App;
use mempalace_rs::split;
use serde_json::{Value, json};

use crate::cli_support::{apply_cli_overrides, palace_exists, print_no_palace};

pub enum ProjectCommand {
    Init {
        dir: PathBuf,
        human: bool,
    },
    Onboarding {
        dir: PathBuf,
        mode: Option<String>,
        people: Vec<String>,
        projects: Vec<String>,
        aliases: Vec<String>,
        wings: Option<String>,
        scan: bool,
        auto_accept_detected: bool,
        human: bool,
    },
    Mine {
        dir: PathBuf,
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
    },
    Search {
        query: String,
        wing: Option<String>,
        room: Option<String>,
        results: usize,
        human: bool,
    },
    Split {
        dir: PathBuf,
        output_dir: Option<PathBuf>,
        min_sessions: usize,
        dry_run: bool,
    },
    Normalize {
        file: PathBuf,
        human: bool,
    },
}

pub async fn handle_project_command(
    command: ProjectCommand,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> Result<()> {
    match command {
        ProjectCommand::Init { dir, human } => {
            let palace_path = palace.unwrap_or(&dir);
            let config = resolve_config(
                Some(palace_path),
                hf_endpoint,
                human,
                print_init_error_human,
                print_init_error_json,
            )?;
            let app = create_app(config, human, print_init_error_human, print_init_error_json)?;
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
                print_json(&summary)?;
            }
        }
        ProjectCommand::Onboarding {
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
                aliases: BTreeMap::new(),
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
                print_json(&summary)?;
            }
        }
        ProjectCommand::Mine {
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
                print_json(&summary)?;
            }
        }
        ProjectCommand::Search {
            query,
            wing,
            room,
            results,
            human,
        } => {
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
        }
        ProjectCommand::Split {
            dir,
            output_dir,
            min_sessions,
            dry_run,
        } => {
            let summary =
                split::split_directory(&dir, output_dir.as_deref(), min_sessions, dry_run)?;
            print_json(&summary)?;
        }
        ProjectCommand::Normalize { file, human } => {
            let raw = std::fs::read_to_string(&file)?;
            let normalized = normalize_conversation_file(&file)?;
            let Some(normalized) = normalized else {
                if human {
                    print_normalize_error_human("Unsupported or unreadable conversation file.");
                } else {
                    print_json(&json!({
                        "error": "Normalize error: Unsupported or unreadable conversation file."
                    }))?;
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

fn print_search_human(summary: &mempalace_rs::model::SearchResults) {
    print!("{}", render_search_human(summary));
}

fn print_normalize_human(summary: &Value) {
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

fn print_init_human(summary: &InitSummary) {
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

fn print_init_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Init error: {message}"),
        "hint": "Check the palace path and SQLite file, then rerun `mempalace-rs init <dir>`.",
    });
    print_json(&payload)
}

fn print_onboarding_human(summary: &OnboardingSummary) {
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

fn print_onboarding_error_json(message: &str) -> Result<()> {
    let payload = json!({
        "error": format!("Onboarding error: {message}"),
        "hint": "Check the project path and onboarding arguments, then rerun `mempalace-rs onboarding <dir>`.",
    });
    print_json(&payload)
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
