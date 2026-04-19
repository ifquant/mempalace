use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use mempalace_rs::config::AppConfig;
use mempalace_rs::hook;
use mempalace_rs::instructions;
use mempalace_rs::mcp;

use crate::cli_support::{apply_cli_overrides, format_mcp_setup};

pub enum HelperCommand {
    Hook { action: HookCommand },
    Instructions { name: String },
    Mcp { setup: bool, serve: bool },
}

#[derive(Subcommand)]
pub enum HookCommand {
    #[command(about = "Execute a hook")]
    Run {
        #[arg(long, help = "Hook name to run")]
        hook: String,
        #[arg(long, help = "Harness type")]
        harness: String,
    },
}

pub async fn handle_helper_command(
    command: HelperCommand,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> Result<()> {
    match command {
        HelperCommand::Hook { action } => {
            let mut config = AppConfig::resolve(palace)?;
            apply_cli_overrides(&mut config, hf_endpoint);
            match action {
                HookCommand::Run {
                    hook: hook_name,
                    harness,
                } => {
                    let output = hook::run_hook(&hook_name, &harness, &config)?;
                    writeln!(
                        std::io::stdout(),
                        "{}",
                        serde_json::to_string_pretty(&output)?
                    )?;
                }
            }
        }
        HelperCommand::Instructions { name } => {
            let text = instructions::render(&name)?;
            print!("{text}");
        }
        HelperCommand::Mcp { setup, serve } => {
            let mut config = AppConfig::resolve(palace)?;
            apply_cli_overrides(&mut config, hf_endpoint);
            if setup || !serve {
                print!("{}", format_mcp_setup(&config.palace_path));
            } else {
                mcp::run_stdio(config).await?;
            }
        }
    }

    Ok(())
}
