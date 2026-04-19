use std::path::PathBuf;

use clap::Subcommand;
use mempalace_rs::config::AppConfig;
use mempalace_rs::model::{
    RegistryConfirmResult, RegistryLearnResult, RegistryLookupResult, RegistryQueryResult,
    RegistryResearchResult, RegistrySummaryResult, RegistryWriteResult,
};
use mempalace_rs::service::App;

use crate::apply_cli_overrides;

#[derive(Subcommand)]
pub enum RegistryCommand {
    #[command(about = "Show a summary of entity_registry.json")]
    Summary {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable registry summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Look up one word in entity_registry.json")]
    Lookup {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Word to look up")]
        word: String,
        #[arg(long, default_value = "")]
        #[arg(help = "Context sentence used for ambiguous-name disambiguation")]
        context: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable lookup output instead of JSON")]
        human: bool,
    },
    #[command(about = "Learn new people/projects from local files into entity_registry.json")]
    Learn {
        #[arg(help = "Project directory to scan and update")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable learn summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Add one person to entity_registry.json")]
    AddPerson {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Person name to add")]
        name: String,
        #[arg(long, default_value = "")]
        #[arg(help = "Relationship or role")]
        relationship: String,
        #[arg(long, default_value = "work")]
        #[arg(help = "Context bucket: work or personal")]
        context: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable write summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Add one project to entity_registry.json")]
    AddProject {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Project name to add")]
        name: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable write summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Add an alias for an existing person in entity_registry.json")]
    AddAlias {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Existing canonical person name")]
        canonical: String,
        #[arg(help = "Alias or nickname to add")]
        alias: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable write summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Extract known people and unknown capitalized candidates from a query")]
    Query {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Query text to inspect")]
        query: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable query summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Research one unknown word into the registry wiki cache")]
    Research {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Word to research via Wikipedia")]
        word: String,
        #[arg(long)]
        #[arg(help = "Mark the researched result as confirmed immediately")]
        auto_confirm: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable research summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Confirm one researched word and promote it into the registry")]
    Confirm {
        #[arg(help = "Project directory containing entity_registry.json")]
        dir: PathBuf,
        #[arg(help = "Word already present in the wiki cache")]
        word: String,
        #[arg(long = "type", default_value = "person")]
        #[arg(help = "Confirmed entity type, usually person")]
        entity_type: String,
        #[arg(long, default_value = "")]
        #[arg(help = "Relationship or role if confirming a person")]
        relationship: String,
        #[arg(long, default_value = "personal")]
        #[arg(help = "Context bucket: work or personal")]
        context: String,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable confirm summary instead of JSON")]
        human: bool,
    },
}

pub fn handle_registry_command(
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
                print_json(&summary)?;
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
                print_json(&summary)?;
            }
        }
        RegistryCommand::Learn { dir, human } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_learn(&dir)?;
            if human {
                print_registry_learn_human(&summary);
            } else {
                print_json(&summary)?;
            }
        }
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
                print_json(&summary)?;
            }
        }
        RegistryCommand::AddProject { dir, name, human } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_add_project(&dir, &name)?;
            if human {
                print_registry_write_human(&summary);
            } else {
                print_json(&summary)?;
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
                print_json(&summary)?;
            }
        }
        RegistryCommand::Query { dir, query, human } => {
            let app = build_registry_app(palace, hf_endpoint)?;
            let summary = app.registry_query(&dir, &query)?;
            if human {
                print_registry_query_human(&summary);
            } else {
                print_json(&summary)?;
            }
        }
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
                print_json(&summary)?;
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
                print_json(&summary)?;
            }
        }
    }

    Ok(())
}

fn build_registry_app(palace: Option<&PathBuf>, hf_endpoint: Option<&str>) -> anyhow::Result<App> {
    let mut config = AppConfig::resolve(palace)?;
    apply_cli_overrides(&mut config, hf_endpoint);
    Ok(App::new(config)?)
}

fn print_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
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
