//! CLI handler for `mempalace-rs onboarding`.
//!
//! It focuses on argument parsing and output formatting while the onboarding
//! module owns prompting, merge rules, and file writes.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use mempalace_rs::model::OnboardingSummary;
use mempalace_rs::onboarding::{
    OnboardingRequest, parse_alias_arg, parse_person_arg, run_onboarding,
};
use serde_json::json;

use crate::project_cli_bootstrap_support::print_bootstrap_json;

#[allow(clippy::too_many_arguments)]
/// Runs onboarding from CLI arguments and prints either human or JSON output.
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
        print_bootstrap_json(&summary)?;
    }
    Ok(())
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
    print_bootstrap_json(&payload)
}
