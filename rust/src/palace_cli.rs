use std::path::PathBuf;

use anyhow::Result;

use crate::palace_cli_embedding::{handle_doctor, handle_prepare_embedding};
use crate::palace_cli_maintenance::{DedupCommand, handle_dedup, handle_migrate, handle_repair};
use crate::palace_cli_read::{
    handle_compress, handle_layers_status, handle_recall, handle_status, handle_wake_up,
};

pub use crate::palace_cli_maintenance::RepairCommand;

pub enum PalaceCommand {
    Compress {
        wing: Option<String>,
        dry_run: bool,
        human: bool,
    },
    WakeUp {
        wing: Option<String>,
        human: bool,
    },
    Recall {
        wing: Option<String>,
        room: Option<String>,
        results: usize,
        human: bool,
    },
    LayersStatus {
        human: bool,
    },
    Migrate {
        human: bool,
    },
    Repair {
        action: Option<RepairCommand>,
        human: bool,
    },
    Dedup {
        threshold: f64,
        dry_run: bool,
        stats: bool,
        wing: Option<String>,
        source: Option<String>,
        human: bool,
    },
    Status {
        human: bool,
    },
    Doctor {
        warm_embedding: bool,
        human: bool,
    },
    PrepareEmbedding {
        attempts: usize,
        wait_ms: u64,
        human: bool,
    },
}

pub async fn handle_palace_command(
    command: PalaceCommand,
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> Result<()> {
    match command {
        PalaceCommand::Compress {
            wing,
            dry_run,
            human,
        } => handle_compress(palace, hf_endpoint, wing, dry_run, human).await,
        PalaceCommand::WakeUp { wing, human } => {
            handle_wake_up(palace, hf_endpoint, wing, human).await
        }
        PalaceCommand::Recall {
            wing,
            room,
            results,
            human,
        } => handle_recall(palace, hf_endpoint, wing, room, results, human).await,
        PalaceCommand::LayersStatus { human } => {
            handle_layers_status(palace, hf_endpoint, human).await
        }
        PalaceCommand::Migrate { human } => handle_migrate(palace, hf_endpoint, human).await,
        PalaceCommand::Repair { action, human } => {
            handle_repair(palace, hf_endpoint, action, human).await
        }
        PalaceCommand::Dedup {
            threshold,
            dry_run,
            stats,
            wing,
            source,
            human,
        } => {
            handle_dedup(
                palace,
                hf_endpoint,
                DedupCommand {
                    threshold,
                    dry_run,
                    stats,
                    wing,
                    source,
                    human,
                },
            )
            .await
        }
        PalaceCommand::Status { human } => handle_status(palace, hf_endpoint, human).await,
        PalaceCommand::Doctor {
            warm_embedding,
            human,
        } => handle_doctor(palace, hf_endpoint, warm_embedding, human).await,
        PalaceCommand::PrepareEmbedding {
            attempts,
            wait_ms,
            human,
        } => handle_prepare_embedding(palace, hf_endpoint, attempts, wait_ms, human).await,
    }
}
