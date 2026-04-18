use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};

use crate::error::{MempalaceError, Result};
use crate::palace::SKIP_DIRS;

const FOLDER_ROOM_MAP: &[(&str, &str)] = &[
    ("frontend", "frontend"),
    ("front-end", "frontend"),
    ("front_end", "frontend"),
    ("client", "frontend"),
    ("ui", "frontend"),
    ("views", "frontend"),
    ("components", "frontend"),
    ("pages", "frontend"),
    ("backend", "backend"),
    ("back-end", "backend"),
    ("back_end", "backend"),
    ("server", "backend"),
    ("api", "backend"),
    ("routes", "backend"),
    ("services", "backend"),
    ("controllers", "backend"),
    ("models", "backend"),
    ("database", "backend"),
    ("db", "backend"),
    ("docs", "documentation"),
    ("doc", "documentation"),
    ("documentation", "documentation"),
    ("wiki", "documentation"),
    ("readme", "documentation"),
    ("notes", "documentation"),
    ("design", "design"),
    ("designs", "design"),
    ("mockups", "design"),
    ("wireframes", "design"),
    ("assets", "design"),
    ("storyboard", "design"),
    ("costs", "costs"),
    ("cost", "costs"),
    ("budget", "costs"),
    ("finance", "costs"),
    ("financial", "costs"),
    ("pricing", "costs"),
    ("invoices", "costs"),
    ("accounting", "costs"),
    ("meetings", "meetings"),
    ("meeting", "meetings"),
    ("calls", "meetings"),
    ("meeting_notes", "meetings"),
    ("standup", "meetings"),
    ("minutes", "meetings"),
    ("team", "team"),
    ("staff", "team"),
    ("hr", "team"),
    ("hiring", "team"),
    ("employees", "team"),
    ("people", "team"),
    ("research", "research"),
    ("references", "research"),
    ("reading", "research"),
    ("papers", "research"),
    ("planning", "planning"),
    ("roadmap", "planning"),
    ("strategy", "planning"),
    ("specs", "planning"),
    ("requirements", "planning"),
    ("tests", "testing"),
    ("test", "testing"),
    ("testing", "testing"),
    ("qa", "testing"),
    ("scripts", "scripts"),
    ("tools", "scripts"),
    ("utils", "scripts"),
    ("config", "configuration"),
    ("configs", "configuration"),
    ("settings", "configuration"),
    ("infrastructure", "configuration"),
    ("infra", "configuration"),
    ("deploy", "configuration"),
];

#[derive(Clone, Debug, Deserialize)]
pub struct ProjectConfig {
    pub wing: Option<String>,
    pub rooms: Option<Vec<ProjectRoom>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ProjectRoom {
    pub name: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoomDetection {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
}

pub fn detect_rooms(project_dir: &Path) -> Result<Vec<RoomDetection>> {
    let mut found_rooms = BTreeMap::new();

    if let Ok(entries) = fs::read_dir(project_dir) {
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                if SKIP_DIRS.contains(&name) {
                    continue;
                }
                let lower = name.to_ascii_lowercase();
                if let Some((_, room_name)) = FOLDER_ROOM_MAP
                    .iter()
                    .find(|(folder, _)| normalize_roomish(&lower) == *folder)
                {
                    found_rooms.insert(
                        room_name.to_string(),
                        RoomDetection {
                            name: room_name.to_string(),
                            description: format!("Files from {name}/"),
                            keywords: vec![room_name.to_string(), lower],
                        },
                    );
                } else if name.len() > 2 && name.chars().next().is_some_and(char::is_alphabetic) {
                    let clean = normalize_roomish(name);
                    found_rooms.entry(clean.clone()).or_insert(RoomDetection {
                        name: clean.clone(),
                        description: format!("Files from {name}/"),
                        keywords: vec![clean],
                    });
                }
            }
        }
    }

    if found_rooms.is_empty() {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for entry in WalkBuilder::new(project_dir)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .parents(true)
            .build()
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
            {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            let normalized = normalize_roomish(&file_name);
            for (keyword, room_name) in FOLDER_ROOM_MAP {
                if normalized.contains(&normalize_roomish(keyword)) {
                    *counts.entry((*room_name).to_string()).or_insert(0) += 1;
                }
            }
        }
        for (room_name, _) in counts.into_iter().filter(|(_, count)| *count >= 2).take(6) {
            found_rooms.insert(
                room_name.clone(),
                RoomDetection {
                    name: room_name.clone(),
                    description: format!("Files related to {room_name}"),
                    keywords: vec![room_name],
                },
            );
        }
    }

    if !found_rooms.contains_key("general") {
        found_rooms.insert(
            "general".to_string(),
            RoomDetection {
                name: "general".to_string(),
                description: "Files that don't fit other rooms".to_string(),
                keywords: Vec::new(),
            },
        );
    }

    Ok(found_rooms.into_values().collect())
}

pub fn detect_room(root: &Path, path: &Path, content: &str, rooms: &[ProjectRoom]) -> String {
    if rooms.is_empty() {
        return "general".to_string();
    }

    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('\\', "/");
    let filename = path
        .file_stem()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let content_lower = content
        .chars()
        .take(2_000)
        .collect::<String>()
        .to_ascii_lowercase();

    for part in relative
        .split('/')
        .filter(|part| !part.is_empty())
        .take_while(|part| !part.contains('.'))
    {
        for room in rooms {
            let mut candidates = vec![room.name.to_ascii_lowercase()];
            candidates.extend(
                room.keywords
                    .iter()
                    .map(|keyword| keyword.to_ascii_lowercase()),
            );
            if candidates.iter().any(|candidate| {
                part == candidate || candidate.contains(part) || part.contains(candidate)
            }) {
                return room.name.clone();
            }
        }
    }

    for room in rooms {
        let room_name = room.name.to_ascii_lowercase();
        if filename.contains(&room_name) || room_name.contains(&filename) {
            return room.name.clone();
        }
    }

    let mut best_room = None;
    let mut best_score = 0;
    for room in rooms {
        let mut score = content_lower
            .matches(&room.name.to_ascii_lowercase())
            .count();
        for keyword in &room.keywords {
            score += content_lower.matches(&keyword.to_ascii_lowercase()).count();
        }
        if score > best_score {
            best_score = score;
            best_room = Some(room.name.clone());
        }
    }

    best_room.unwrap_or_else(|| "general".to_string())
}

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

fn normalize_roomish(value: &str) -> String {
    value.to_ascii_lowercase().replace(['-', ' '], "_")
}

#[cfg(test)]
mod tests {
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
