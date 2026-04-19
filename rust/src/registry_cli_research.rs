use std::path::PathBuf;

use mempalace_rs::model::{RegistryConfirmResult, RegistryResearchResult};

use crate::registry_cli::RegistryCommand;
use crate::registry_cli_support::{build_registry_app, print_registry_json};

pub fn handle_registry_research_command(
    action: RegistryCommand,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> anyhow::Result<()> {
    match action {
        RegistryCommand::Research {
            dir,
            word,
            auto_confirm,
            human,
        } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_research(&dir, &word, auto_confirm)?;
            if human {
                print_registry_research_human(&summary);
            } else {
                print_registry_json(&summary)?;
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
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary =
                app.registry_confirm_research(&dir, &word, &entity_type, &relationship, &context)?;
            if human {
                print_registry_confirm_human(&summary);
            } else {
                print_registry_json(&summary)?;
            }
        }
        _ => unreachable!("non-research registry command routed to research handler"),
    }

    Ok(())
}

fn print_registry_research_human(summary: &RegistryResearchResult) {
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

fn print_registry_confirm_human(summary: &RegistryConfirmResult) {
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
