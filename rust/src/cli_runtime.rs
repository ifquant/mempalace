//! CLI dispatcher that bridges `root_cli` into the split runtime families.
//!
//! This file is the high-level routing layer for the binary: it destructures
//! root flags once, then forwards each command into the project, palace,
//! helper, or registry handlers.

use anyhow::Result;

use crate::helper_cli::{HelperCommand, handle_helper_command};
use crate::palace_cli::{PalaceCommand, handle_palace_command};
use crate::project_cli::{ProjectCommand, handle_project_command};
use crate::registry_cli::handle_registry_command;
use crate::root_cli::{Cli, Command};

/// Run one parsed CLI invocation against the appropriate command family.
pub async fn run_cli(cli: Cli) -> Result<()> {
    let Cli {
        palace,
        hf_endpoint,
        command,
    } = cli;

    match command {
        Command::Init { dir, yes: _, human } => {
            handle_project_command(
                ProjectCommand::Init { dir, human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Onboarding {
            dir,
            mode,
            people,
            projects,
            aliases,
            wings,
            scan,
            auto_accept_detected,
            human,
        } => {
            handle_project_command(
                ProjectCommand::Onboarding {
                    dir,
                    mode,
                    people,
                    projects,
                    aliases,
                    wings,
                    scan,
                    auto_accept_detected,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Mine {
            dir,
            mode,
            wing,
            limit,
            dry_run,
            no_gitignore,
            include_ignored,
            agent,
            extract,
            progress,
            human,
        } => {
            handle_project_command(
                ProjectCommand::Mine {
                    dir,
                    mode,
                    wing,
                    limit,
                    dry_run,
                    no_gitignore,
                    include_ignored,
                    agent,
                    extract,
                    progress,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Search {
            query,
            wing,
            room,
            results,
            human,
        } => {
            handle_project_command(
                ProjectCommand::Search {
                    query,
                    wing,
                    room,
                    results,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Split {
            dir,
            source,
            file,
            output_dir,
            min_sessions,
            dry_run,
        } => {
            handle_project_command(
                ProjectCommand::Split {
                    dir,
                    source,
                    file,
                    output_dir,
                    min_sessions,
                    dry_run,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Normalize { file, human } => {
            handle_project_command(
                ProjectCommand::Normalize { file, human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Compress {
            wing,
            dry_run,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::Compress {
                    wing,
                    dry_run,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::WakeUp { wing, human } => {
            handle_palace_command(
                PalaceCommand::WakeUp { wing, human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Recall {
            wing,
            room,
            results,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::Recall {
                    wing,
                    room,
                    results,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::LayersStatus { human } => {
            handle_palace_command(
                PalaceCommand::LayersStatus { human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Migrate { human } => {
            handle_palace_command(
                PalaceCommand::Migrate { human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Repair { action, human } => {
            handle_palace_command(
                PalaceCommand::Repair { action, human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Dedup {
            threshold,
            dry_run,
            stats,
            wing,
            source,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::Dedup {
                    threshold,
                    dry_run,
                    stats,
                    wing,
                    source,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Status { human } => {
            handle_palace_command(
                PalaceCommand::Status { human },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Doctor {
            warm_embedding,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::Doctor {
                    warm_embedding,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::PrepareEmbedding {
            attempts,
            wait_ms,
            human,
        } => {
            handle_palace_command(
                PalaceCommand::PrepareEmbedding {
                    attempts,
                    wait_ms,
                    human,
                },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Hook { action } => {
            handle_helper_command(
                HelperCommand::Hook { action },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Instructions { name } => {
            handle_helper_command(
                HelperCommand::Instructions { name },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
        Command::Registry { action } => {
            handle_registry_command(action, palace.as_ref(), hf_endpoint.as_deref())?;
        }
        Command::Mcp { setup, serve } => {
            handle_helper_command(
                HelperCommand::Mcp { setup, serve },
                palace.as_ref(),
                hf_endpoint.as_deref(),
            )
            .await?;
        }
    }

    Ok(())
}
