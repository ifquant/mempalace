use clap::Parser;

mod cli_runtime;
mod cli_support;
mod helper_cli;
mod palace_cli;
mod palace_cli_embedding;
mod palace_cli_maintenance;
mod palace_cli_read;
mod palace_cli_support;
mod project_cli;
mod registry_cli;
mod root_cli;

use cli_runtime::run_cli;
use root_cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_cli(Cli::parse()).await
}
