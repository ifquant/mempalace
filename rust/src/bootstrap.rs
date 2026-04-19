use std::path::Path;

use crate::bootstrap_docs::write_entity_registry;
use crate::bootstrap_files::{load_existing_entities, load_existing_rooms, write_project_config};
use crate::entity_detector::detect_entities;
use crate::error::{MempalaceError, Result};
use crate::room_detector::detect_rooms;

pub use crate::bootstrap_docs::{write_aaak_entities, write_critical_facts};
pub use crate::bootstrap_files::{write_entities, write_project_config_from_names};

#[derive(Clone, Debug, PartialEq)]
pub struct InitBootstrap {
    pub wing: String,
    pub configured_rooms: Vec<String>,
    pub detected_people: Vec<String>,
    pub detected_projects: Vec<String>,
    pub config_path: Option<String>,
    pub config_written: bool,
    pub entities_path: Option<String>,
    pub entities_written: bool,
    pub entity_registry_path: Option<String>,
    pub entity_registry_written: bool,
    pub aaak_entities_path: Option<String>,
    pub aaak_entities_written: bool,
    pub critical_facts_path: Option<String>,
    pub critical_facts_written: bool,
}

pub fn bootstrap_project(project_dir: &Path) -> Result<InitBootstrap> {
    let project_dir = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());
    if !project_dir.exists() {
        return Err(MempalaceError::InvalidArgument(format!(
            "Project directory does not exist: {}",
            project_dir.display()
        )));
    }

    let default_wing = default_wing(&project_dir);
    let config_path = project_dir.join("mempalace.yaml");
    let entities_path = project_dir.join("entities.json");

    let (wing, configured_rooms, config_written) = if config_path.exists() {
        let (existing_wing, rooms) = load_existing_rooms(&config_path, &default_wing)?;
        (existing_wing, rooms, false)
    } else {
        let rooms = detect_rooms(&project_dir)?;
        write_project_config(&config_path, &default_wing, &rooms)?;
        (
            default_wing.clone(),
            rooms.into_iter().map(|room| room.name).collect(),
            true,
        )
    };

    let (detected_people, detected_projects, entities_written) = if entities_path.exists() {
        let existing = load_existing_entities(&entities_path)?;
        (existing.people, existing.projects, false)
    } else {
        let detected = detect_entities(&project_dir)?;
        if detected.people.is_empty() && detected.projects.is_empty() {
            (Vec::new(), Vec::new(), false)
        } else {
            write_entities(&entities_path, &detected.people, &detected.projects)?;
            (detected.people, detected.projects, true)
        }
    };

    let entity_registry_path = project_dir.join("entity_registry.json");
    let entity_registry_written = if entity_registry_path.exists() {
        false
    } else {
        write_entity_registry(
            &entity_registry_path,
            &detected_people,
            &detected_projects,
            "work",
        )?;
        true
    };

    let aaak_entities_path = project_dir.join("aaak_entities.md");
    let critical_facts_path = project_dir.join("critical_facts.md");
    let aaak_entities_written = if aaak_entities_path.exists() {
        false
    } else {
        write_aaak_entities(
            &aaak_entities_path,
            &detected_people,
            &detected_projects,
            "work",
        )?;
        true
    };
    let critical_facts_written = if critical_facts_path.exists() {
        false
    } else {
        write_critical_facts(
            &critical_facts_path,
            &detected_people,
            &detected_projects,
            &configured_rooms,
            &wing,
            "work",
        )?;
        true
    };

    Ok(InitBootstrap {
        wing,
        configured_rooms,
        detected_people,
        detected_projects,
        config_path: Some(config_path.display().to_string()),
        config_written,
        entities_path: if entities_path.exists() || entities_written {
            Some(entities_path.display().to_string())
        } else {
            None
        },
        entities_written,
        entity_registry_path: Some(entity_registry_path.display().to_string()),
        entity_registry_written,
        aaak_entities_path: Some(aaak_entities_path.display().to_string()),
        aaak_entities_written,
        critical_facts_path: Some(critical_facts_path.display().to_string()),
        critical_facts_written,
    })
}

pub fn default_wing(project_dir: &Path) -> String {
    project_dir
        .file_name()
        .map(|name| {
            name.to_string_lossy()
                .to_ascii_lowercase()
                .replace([' ', '-'], "_")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "project".to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tempfile::tempdir;

    #[test]
    fn bootstrap_detects_rooms_and_entities_and_writes_files() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("frontend")).unwrap();
        fs::write(
            project.join("notes.md"),
            "Jordan said the Atlas service should launch next week.\nJordan wrote the Atlas architecture guide.",
        )
        .unwrap();

        let result = bootstrap_project(&project).unwrap();
        assert_eq!(result.wing, "project");
        assert!(result.config_written);
        assert!(result.entities_written);
        assert!(result.aaak_entities_written);
        assert!(result.critical_facts_written);
        assert!(
            result
                .configured_rooms
                .iter()
                .any(|room| room == "frontend")
        );
        assert!(result.detected_people.iter().any(|name| name == "Jordan"));
        assert!(result.detected_projects.iter().any(|name| name == "Atlas"));
        assert!(project.join("mempalace.yaml").exists());
        assert!(project.join("entities.json").exists());
        assert!(project.join("aaak_entities.md").exists());
        assert!(project.join("critical_facts.md").exists());
    }

    #[test]
    fn bootstrap_preserves_existing_files() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("mempalace.yaml"),
            "wing: custom\nrooms:\n  - name: docs\n",
        )
        .unwrap();
        fs::write(
            project.join("entities.json"),
            r#"{"people":["Riley"],"projects":["MemPalace"]}"#,
        )
        .unwrap();

        let result = bootstrap_project(&project).unwrap();
        assert_eq!(result.configured_rooms, vec!["docs"]);
        assert_eq!(result.detected_people, vec!["Riley"]);
        assert_eq!(result.detected_projects, vec!["MemPalace"]);
        assert!(!result.config_written);
        assert!(!result.entities_written);
        assert!(result.aaak_entities_written);
        assert!(result.critical_facts_written);
    }
}
