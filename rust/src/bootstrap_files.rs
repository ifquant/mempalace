use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{MempalaceError, Result};
use crate::room_detector::RoomDetection;

#[derive(Clone, Debug, Deserialize)]
struct ExistingProjectConfig {
    wing: Option<String>,
    rooms: Option<Vec<ExistingRoom>>,
}

#[derive(Clone, Debug, Deserialize)]
struct ExistingRoom {
    name: String,
}

#[derive(Clone, Debug, Serialize)]
struct GeneratedProjectConfig {
    wing: String,
    rooms: Vec<GeneratedRoom>,
}

#[derive(Clone, Debug, Serialize)]
struct GeneratedRoom {
    name: String,
    description: String,
    keywords: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExistingEntities {
    #[serde(default)]
    pub people: Vec<String>,
    #[serde(default)]
    pub projects: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct GeneratedEntities {
    people: Vec<String>,
    projects: Vec<String>,
}

pub fn load_existing_rooms(
    config_path: &Path,
    fallback_wing: &str,
) -> Result<(String, Vec<String>)> {
    let content = fs::read_to_string(config_path)?;
    let config = serde_yml::from_str::<ExistingProjectConfig>(&content).map_err(|err| {
        MempalaceError::InvalidArgument(format!(
            "Failed to parse existing project config {}: {err}",
            config_path.display()
        ))
    })?;
    let mut rooms = config
        .rooms
        .unwrap_or_default()
        .into_iter()
        .map(|room| room.name)
        .filter(|name| !name.trim().is_empty())
        .collect::<Vec<_>>();
    if rooms.is_empty() {
        rooms.push("general".to_string());
    }
    let wing = config.wing.unwrap_or_else(|| fallback_wing.to_string());
    Ok((wing, rooms))
}

pub fn write_project_config(config_path: &Path, wing: &str, rooms: &[RoomDetection]) -> Result<()> {
    let config = GeneratedProjectConfig {
        wing: wing.to_string(),
        rooms: rooms
            .iter()
            .map(|room| GeneratedRoom {
                name: room.name.clone(),
                description: room.description.clone(),
                keywords: room.keywords.clone(),
            })
            .collect(),
    };
    let content = serde_yml::to_string(&config).map_err(|err| {
        MempalaceError::InvalidArgument(format!(
            "Failed to render project config {}: {err}",
            config_path.display()
        ))
    })?;
    fs::write(config_path, content)?;
    Ok(())
}

pub fn write_project_config_from_names(
    config_path: &Path,
    wing: &str,
    room_names: &[String],
) -> Result<()> {
    let mut rooms = room_names
        .iter()
        .filter_map(|name| {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(RoomDetection {
                    name: trimmed.to_string(),
                    description: format!("Files related to {trimmed}"),
                    keywords: vec![trimmed.to_string()],
                })
            }
        })
        .collect::<Vec<_>>();
    if rooms.is_empty() {
        rooms.push(RoomDetection {
            name: "general".to_string(),
            description: "Files that don't fit other rooms".to_string(),
            keywords: Vec::new(),
        });
    }
    write_project_config(config_path, wing, &rooms)
}

pub fn load_existing_entities(entities_path: &Path) -> Result<ExistingEntities> {
    let content = fs::read_to_string(entities_path)?;
    serde_json::from_str::<ExistingEntities>(&content).map_err(|err| {
        MempalaceError::InvalidArgument(format!(
            "Failed to parse existing entities file {}: {err}",
            entities_path.display()
        ))
    })
}

pub fn write_entities(entities_path: &Path, people: &[String], projects: &[String]) -> Result<()> {
    let payload = GeneratedEntities {
        people: people.to_vec(),
        projects: projects.to_vec(),
    };
    let content = serde_json::to_string_pretty(&payload)?;
    fs::write(entities_path, content)?;
    Ok(())
}
