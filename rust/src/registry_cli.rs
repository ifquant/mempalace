//! CLI command tree for registry operations.
//!
//! Audit readers can start here to see the user-visible registry surfaces, then
//! follow the routed handlers for read, write, and research behavior.

use std::path::PathBuf;

use clap::Subcommand;

use crate::registry_cli_read::handle_registry_read_command;
use crate::registry_cli_research::handle_registry_research_command;
use crate::registry_cli_write::handle_registry_write_command;

#[derive(Subcommand)]
/// Registry-related CLI subcommands grouped by read/write/research intent.
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

/// Routes a parsed registry command to the matching handler family.
pub fn handle_registry_command(
    action: RegistryCommand,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> anyhow::Result<()> {
    match action {
        RegistryCommand::Summary { .. }
        | RegistryCommand::Lookup { .. }
        | RegistryCommand::Learn { .. }
        | RegistryCommand::Query { .. } => {
            handle_registry_read_command(action, palace, hf_endpoint)
        }
        RegistryCommand::AddPerson { .. }
        | RegistryCommand::AddProject { .. }
        | RegistryCommand::AddAlias { .. } => {
            handle_registry_write_command(action, palace, hf_endpoint)
        }
        RegistryCommand::Research { .. } | RegistryCommand::Confirm { .. } => {
            handle_registry_research_command(action, palace, hf_endpoint)
        }
    }
}
