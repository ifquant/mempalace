//! Project room-detection facade.
//!
//! These helpers load explicit room config when present and otherwise build a
//! lightweight room map from folder structure and filename/content heuristics.

use serde::{Deserialize, Serialize};

#[path = "room_detector_config.rs"]
mod config;
#[path = "room_detector_detect.rs"]
mod detect;

pub use config::{load_project_config, load_project_rooms};
pub use detect::{detect_room, detect_rooms};

#[derive(Clone, Debug, Deserialize)]
/// Optional project-level mining configuration loaded from YAML.
pub struct ProjectConfig {
    pub wing: Option<String>,
    pub rooms: Option<Vec<ProjectRoom>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
/// One configured room plus the keywords used to match it.
pub struct ProjectRoom {
    pub name: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
/// Auto-detected room description returned by discovery flows.
pub struct RoomDetection {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use tempfile::tempdir;

    #[test]
    fn detect_rooms_prefers_folder_structure_and_falls_back_to_general() {
        let tmp = tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("frontend")).unwrap();
        fs::create_dir_all(tmp.path().join("notes")).unwrap();

        let rooms = detect_rooms(tmp.path()).unwrap();
        let names = rooms.into_iter().map(|room| room.name).collect::<Vec<_>>();
        assert!(names.iter().any(|name| name == "frontend"));
        assert!(names.iter().any(|name| name == "documentation"));
        assert!(names.iter().any(|name| name == "general"));
    }

    #[test]
    fn detect_room_uses_path_and_keyword_rules() {
        let root = Path::new("/tmp/project");
        let path = Path::new("/tmp/project/src/security.txt");
        let rooms = vec![
            ProjectRoom {
                name: "auth".to_string(),
                keywords: vec!["jwt".to_string(), "token".to_string()],
            },
            ProjectRoom {
                name: "docs".to_string(),
                keywords: vec!["guide".to_string()],
            },
        ];
        assert_eq!(
            detect_room(root, path, "JWT token handling and auth flows", &rooms),
            "auth"
        );
    }
}
