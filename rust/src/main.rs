use clap::Parser;

mod cli_runtime;
mod cli_support;
mod helper_cli;
mod palace_cli;
mod palace_cli_dedup;
mod palace_cli_embedding;
mod palace_cli_embedding_doctor;
mod palace_cli_embedding_prepare;
mod palace_cli_embedding_support;
mod palace_cli_maintenance;
mod palace_cli_maintenance_support;
mod palace_cli_migrate;
mod palace_cli_read;
mod palace_cli_read_compress;
mod palace_cli_read_layers;
mod palace_cli_read_status;
mod palace_cli_read_support;
mod palace_cli_repair;
mod palace_cli_support;
mod project_cli;
mod project_cli_bootstrap;
mod project_cli_mining;
mod project_cli_support;
mod project_cli_transcript;
mod registry_cli;
mod registry_cli_read;
mod registry_cli_research;
mod registry_cli_support;
mod registry_cli_write;
mod root_cli;

use cli_runtime::run_cli;
use root_cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_cli(Cli::parse()).await
}
