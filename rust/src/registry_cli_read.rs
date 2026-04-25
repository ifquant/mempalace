//! CLI handlers for read-only registry commands.
//!
//! These commands all build the same `App` facade, then choose human or JSON
//! rendering for the corresponding registry read result.

use std::path::PathBuf;

use mempalace_rs::model::{
    RegistryLearnResult, RegistryLookupResult, RegistryQueryResult, RegistrySummaryResult,
};

use crate::registry_cli::RegistryCommand;
use crate::registry_cli_support::{build_registry_app, print_registry_json};

/// Executes registry read commands such as summary, lookup, learn, and query.
pub fn handle_registry_read_command(
    action: RegistryCommand,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> anyhow::Result<()> {
    match action {
        RegistryCommand::Summary { dir, human } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_summary(&dir)?;
            if human {
                print_registry_summary_human(&summary);
            } else {
                print_registry_json(&summary)?;
            }
        }
        RegistryCommand::Lookup {
            dir,
            word,
            context,
            human,
        } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_lookup(&dir, &word, &context)?;
            if human {
                print_registry_lookup_human(&summary);
            } else {
                print_registry_json(&summary)?;
            }
        }
        RegistryCommand::Learn { dir, human } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_learn(&dir)?;
            if human {
                print_registry_learn_human(&summary);
            } else {
                print_registry_json(&summary)?;
            }
        }
        RegistryCommand::Query { dir, query, human } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_query(&dir, &query)?;
            if human {
                print_registry_query_human(&summary);
            } else {
                print_registry_json(&summary)?;
            }
        }
        _ => unreachable!("non-read registry command routed to read handler"),
    }

    Ok(())
}

fn print_registry_summary_human(summary: &RegistrySummaryResult) {
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

fn print_registry_lookup_human(summary: &RegistryLookupResult) {
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

fn print_registry_learn_human(summary: &RegistryLearnResult) {
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

fn print_registry_query_human(summary: &RegistryQueryResult) {
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
