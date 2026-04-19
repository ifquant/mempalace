use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use mempalace_rs::model::{InitSummary, OnboardingSummary};
use mempalace_rs::onboarding::{
    OnboardingRequest, parse_alias_arg, parse_person_arg, run_onboarding,
};
use serde_json::json;

use crate::project_cli_support::{create_app, print_json, resolve_config};

pub async fn handle_init(
    dir: &Path,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
) -> Result<()> {
    let palace_path = palace.cloned().unwrap_or_else(|| dir.to_path_buf());
    let config = resolve_config(
        Some(&palace_path),
        hf_endpoint,
        human,
        print_init_error_human,
        print_init_error_json,
    )?;
    let app = create_app(config, human, print_init_error_human, print_init_error_json)?;
    let summary = match app.init_project(dir).await {
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
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_onboarding(
    dir: &Path,
    mode: Option<String>,
    people: Vec<String>,
    projects: Vec<String>,
    aliases: Vec<String>,
    wings: Option<String>,
    scan: bool,
    auto_accept_detected: bool,
    human: bool,
) -> Result<()> {
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

    let summary = match run_onboarding(dir, request) {
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
    Ok(())
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
