use std::fs;
use std::path::Path;

use crate::error::{MempalaceError, Result};

use super::{ProjectConfig, ProjectRoom};

/// Loads the room list for a project, defaulting to a single `general` room.
pub fn load_project_rooms(project_dir: &Path) -> Result<Vec<ProjectRoom>> {
    let config = load_project_config(project_dir)?;
    Ok(config
        .and_then(|config| config.rooms)
        .filter(|rooms| !rooms.is_empty())
        .unwrap_or_else(|| {
            vec![ProjectRoom {
                name: "general".to_string(),
                keywords: Vec::new(),
            }]
        }))
}

/// Loads project mining configuration from `mempalace.yaml` or `mempal.yaml`.
pub fn load_project_config(project_dir: &Path) -> Result<Option<ProjectConfig>> {
    for name in ["mempalace.yaml", "mempal.yaml"] {
        let path = project_dir.join(name);
        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(path)?;
        let config = serde_yml::from_str::<ProjectConfig>(&content).map_err(|err| {
            MempalaceError::InvalidArgument(format!("Invalid project config: {err}"))
        })?;
        return Ok(Some(config));
    }

    Ok(None)
}
