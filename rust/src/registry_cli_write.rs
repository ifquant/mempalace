use std::path::PathBuf;

use mempalace_rs::model::RegistryWriteResult;

use crate::registry_cli::RegistryCommand;
use crate::registry_cli_support::{build_registry_app, print_registry_json};

pub fn handle_registry_write_command(
    action: RegistryCommand,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> anyhow::Result<()> {
    match action {
        RegistryCommand::AddPerson {
            dir,
            name,
            relationship,
            context,
            human,
        } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_add_person(&dir, &name, &relationship, &context)?;
            if human {
                print_registry_write_human(&summary);
            } else {
                print_registry_json(&summary)?;
            }
        }
        RegistryCommand::AddProject { dir, name, human } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_add_project(&dir, &name)?;
            if human {
                print_registry_write_human(&summary);
            } else {
                print_registry_json(&summary)?;
            }
        }
        RegistryCommand::AddAlias {
            dir,
            canonical,
            alias,
            human,
        } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_add_alias(&dir, &canonical, &alias)?;
            if human {
                print_registry_write_human(&summary);
            } else {
                print_registry_json(&summary)?;
            }
        }
        _ => unreachable!("non-write registry command routed to write handler"),
    }

    Ok(())
}

fn print_registry_write_human(summary: &RegistryWriteResult) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Registry Write");
    println!("{}\n", "=".repeat(55));
    println!("  Registry: {}", summary.registry_path);
    println!("  Action: {}", summary.action);
    println!("  Name: {}", summary.name);
    if let Some(canonical) = &summary.canonical {
        println!("  Canonical: {canonical}");
    }
    println!("  Mode: {}", summary.mode);
    println!(
        "  Totals: {} people, {} projects",
        summary.people_count, summary.project_count
    );
    println!("\n{}", "=".repeat(55));
    println!();
}
