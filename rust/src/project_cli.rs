use std::path::PathBuf;

use anyhow::Result;

use crate::project_cli_bootstrap::{handle_init, handle_onboarding};
use crate::project_cli_mining::{handle_mine, handle_search};
use crate::project_cli_transcript::{handle_normalize, handle_split};

pub enum ProjectCommand {
    Init {
        dir: PathBuf,
        human: bool,
    },
    Onboarding {
        dir: PathBuf,
        mode: Option<String>,
        people: Vec<String>,
        projects: Vec<String>,
        aliases: Vec<String>,
        wings: Option<String>,
        scan: bool,
        auto_accept_detected: bool,
        human: bool,
    },
    Mine {
        dir: PathBuf,
        mode: String,
        wing: Option<String>,
        limit: usize,
        dry_run: bool,
        no_gitignore: bool,
        include_ignored: Vec<String>,
        agent: String,
        extract: String,
        progress: bool,
        human: bool,
    },
    Search {
        query: String,
        wing: Option<String>,
        room: Option<String>,
        results: usize,
        human: bool,
    },
    Split {
        dir: Option<PathBuf>,
        source: Option<PathBuf>,
        file: Option<PathBuf>,
        output_dir: Option<PathBuf>,
        min_sessions: usize,
        dry_run: bool,
    },
    Normalize {
        file: PathBuf,
        human: bool,
    },
}

pub async fn handle_project_command(
    command: ProjectCommand,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> Result<()> {
    match command {
        ProjectCommand::Init { dir, human } => handle_init(&dir, palace, hf_endpoint, human).await,
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
        } => {
            handle_onboarding(
                &dir,
                mode,
                people,
                projects,
                aliases,
                wings,
                scan,
                auto_accept_detected,
                human,
            )
            .await
        }
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
        } => {
            handle_mine(
                &dir,
                palace,
                hf_endpoint,
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
            )
            .await
        }
        ProjectCommand::Search {
            query,
            wing,
            room,
            results,
            human,
        } => handle_search(palace, hf_endpoint, query, wing, room, results, human).await,
        ProjectCommand::Split {
            dir,
            source,
            file,
            output_dir,
            min_sessions,
            dry_run,
        } => handle_split(
            dir.as_deref(),
            source.as_deref(),
            file.as_deref(),
            output_dir.as_deref(),
            min_sessions,
            dry_run,
        ),
        ProjectCommand::Normalize { file, human } => handle_normalize(&file, human),
    }
}
