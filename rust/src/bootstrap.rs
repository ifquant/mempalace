use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{MempalaceError, Result};
use crate::registry::EntityRegistry;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "env",
    "dist",
    "build",
    ".next",
    "coverage",
    ".mempalace",
    ".ruff_cache",
    ".mypy_cache",
    ".pytest_cache",
    ".cache",
    ".tox",
    ".nox",
    ".idea",
    ".vscode",
    ".ipynb_checkpoints",
    ".eggs",
    "htmlcov",
    "target",
];

const PROSE_EXTENSIONS: &[&str] = &[".txt", ".md", ".rst", ".csv", ".json", ".jsonl"];
const READABLE_EXTENSIONS: &[&str] = &[
    ".txt", ".md", ".rst", ".csv", ".json", ".jsonl", ".yaml", ".yml", ".toml",
];

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "that", "this", "there", "their", "they", "then", "have",
    "has", "had", "when", "where", "which", "your", "about", "into", "could", "should", "would",
    "return", "import", "class", "usage", "step", "check", "build", "deploy", "project", "system",
    "service", "feature", "issue", "design", "notes", "graph", "search",
];

const PERSON_VERBS: &[&str] = &[
    "said", "asked", "told", "replied", "laughed", "smiled", "cried", "felt", "thinks", "think",
    "wants", "want", "loves", "love", "hates", "hate", "knows", "know", "decided", "pushed",
    "wrote",
];

const PROJECT_HINTS: &[&str] = &[
    "repo",
    "system",
    "service",
    "app",
    "architecture",
    "pipeline",
    "deploy",
    "deployment",
    "launch",
    "shipping",
    "shipped",
    "building",
    "built",
];

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
struct ExistingEntities {
    #[serde(default)]
    people: Vec<String>,
    #[serde(default)]
    projects: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct GeneratedEntities {
    people: Vec<String>,
    projects: Vec<String>,
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

#[derive(Clone, Debug)]
struct RoomDetection {
    name: String,
    description: String,
    keywords: Vec<String>,
}

#[derive(Clone, Debug)]
struct DetectedEntities {
    people: Vec<String>,
    projects: Vec<String>,
}

pub fn detect_entities_for_registry(project_dir: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let detected = detect_entities(project_dir)?;
    Ok((detected.people, detected.projects))
}

fn default_wing(project_dir: &Path) -> String {
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

fn load_existing_rooms(config_path: &Path, fallback_wing: &str) -> Result<(String, Vec<String>)> {
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

fn write_project_config(config_path: &Path, wing: &str, rooms: &[RoomDetection]) -> Result<()> {
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

fn load_existing_entities(entities_path: &Path) -> Result<ExistingEntities> {
    let content = fs::read_to_string(entities_path)?;
    serde_json::from_str::<ExistingEntities>(&content).map_err(|err| {
        MempalaceError::InvalidArgument(format!(
            "Failed to parse existing entities file {}: {err}",
            entities_path.display()
        ))
    })
}

fn write_entities(entities_path: &Path, people: &[String], projects: &[String]) -> Result<()> {
    let payload = GeneratedEntities {
        people: people.to_vec(),
        projects: projects.to_vec(),
    };
    let content = serde_json::to_string_pretty(&payload)?;
    fs::write(entities_path, content)?;
    Ok(())
}

fn write_entity_registry(
    entity_registry_path: &Path,
    people: &[String],
    projects: &[String],
    mode: &str,
) -> Result<()> {
    let registry = EntityRegistry::bootstrap(mode, people, projects);
    registry.save(entity_registry_path)
}

fn write_aaak_entities(
    aaak_entities_path: &Path,
    people: &[String],
    projects: &[String],
    mode: &str,
) -> Result<()> {
    let mut registry_lines = vec![
        "# AAAK Entity Registry".to_string(),
        "# Auto-generated by mempalace-rs init. Update as needed.".to_string(),
        String::new(),
        format!("Mode: {mode}"),
        String::new(),
        "## People".to_string(),
    ];

    for person in people {
        registry_lines.push(format!("  {}={person}", entity_code(person, 3)));
    }
    if people.is_empty() {
        registry_lines.push("  (none detected yet)".to_string());
    }

    registry_lines.push(String::new());
    registry_lines.push("## Projects".to_string());
    for project in projects {
        registry_lines.push(format!("  {}={project}", entity_code(project, 4)));
    }
    if projects.is_empty() {
        registry_lines.push("  (none detected yet)".to_string());
    }

    registry_lines.extend([
        String::new(),
        "## AAAK Quick Reference".to_string(),
        "  Symbols: ♡=love ★=importance ⚠=warning →=relationship |=separator".to_string(),
        "  Structure: KEY:value | GROUP(details) | entity.attribute".to_string(),
        "  Read naturally — expand codes, treat *markers* as emotional context.".to_string(),
    ]);

    fs::write(aaak_entities_path, registry_lines.join("\n"))?;
    Ok(())
}

fn write_critical_facts(
    critical_facts_path: &Path,
    people: &[String],
    projects: &[String],
    configured_rooms: &[String],
    wing: &str,
    mode: &str,
) -> Result<()> {
    let mut lines = vec![
        "# Critical Facts (bootstrap — will be enriched after mining)".to_string(),
        String::new(),
        "## People".to_string(),
    ];

    for person in people {
        lines.push(format!("- **{person}** ({})", entity_code(person, 3)));
    }
    if people.is_empty() {
        lines.push("- none detected yet".to_string());
    }

    lines.push(String::new());
    lines.push("## Projects".to_string());
    for project in projects {
        lines.push(format!("- **{project}**"));
    }
    if projects.is_empty() {
        lines.push("- none detected yet".to_string());
    }

    lines.extend([
        String::new(),
        "## Palace".to_string(),
        format!("Wing: {wing}"),
        format!("Rooms: {}", configured_rooms.join(", ")),
        format!("Mode: {mode}"),
        String::new(),
        "*This file will be enriched after mining.*".to_string(),
    ]);

    fs::write(critical_facts_path, lines.join("\n"))?;
    Ok(())
}

fn entity_code(value: &str, max_len: usize) -> String {
    let mut cleaned = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    cleaned.truncate(max_len);
    while cleaned.len() < max_len {
        cleaned.push('X');
    }
    cleaned
}

fn detect_rooms(project_dir: &Path) -> Result<Vec<RoomDetection>> {
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
                keywords: vec![],
            },
        );
    }

    Ok(found_rooms.into_values().collect())
}

fn detect_entities(project_dir: &Path) -> Result<DetectedEntities> {
    let files = scan_for_detection(project_dir)?;
    if files.is_empty() {
        return Ok(DetectedEntities {
            people: Vec::new(),
            projects: Vec::new(),
        });
    }

    let mut all_text = String::new();
    let mut all_lines = Vec::new();
    let max_bytes = 5_000usize;

    for path in files.into_iter().take(10) {
        let mut content = fs::read_to_string(&path).unwrap_or_default();
        if content.len() > max_bytes {
            content.truncate(max_bytes);
        }
        all_lines.extend(content.lines().map(|line| line.to_string()));
        all_text.push_str(&content);
        all_text.push('\n');
    }

    let candidate_re = Regex::new(r"\b[A-Z][a-z]{2,}\b").unwrap();
    let mut counts = BTreeMap::<String, usize>::new();
    for cap in candidate_re.captures_iter(&all_text) {
        let name = cap.get(0).unwrap().as_str();
        if STOPWORDS.iter().any(|word| word.eq_ignore_ascii_case(name)) {
            continue;
        }
        *counts.entry(name.to_string()).or_insert(0) += 1;
    }

    let mut people = Vec::new();
    let mut projects = Vec::new();
    for (name, frequency) in counts {
        if frequency < 2 {
            continue;
        }
        let person_score = score_person(&name, &all_text, &all_lines);
        let project_score = score_project(&name, &all_text, &all_lines);
        if person_score >= 2 && person_score >= project_score {
            people.push((name, person_score, frequency));
        } else if project_score >= 2 {
            projects.push((name, project_score, frequency));
        }
    }

    people.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then(right.2.cmp(&left.2))
            .then(left.0.cmp(&right.0))
    });
    projects.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then(right.2.cmp(&left.2))
            .then(left.0.cmp(&right.0))
    });

    Ok(DetectedEntities {
        people: people
            .into_iter()
            .map(|(name, _, _)| name)
            .take(15)
            .collect(),
        projects: projects
            .into_iter()
            .map(|(name, _, _)| name)
            .take(10)
            .collect(),
    })
}

fn scan_for_detection(project_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut prose_files = Vec::new();
    let mut readable_files = Vec::new();
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
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!(".{}", value.to_ascii_lowercase()))
            .unwrap_or_default();
        if PROSE_EXTENSIONS.contains(&ext.as_str()) {
            prose_files.push(path.to_path_buf());
        } else if READABLE_EXTENSIONS.contains(&ext.as_str()) {
            readable_files.push(path.to_path_buf());
        }
    }
    let files = if prose_files.len() >= 3 {
        prose_files
    } else {
        prose_files
            .into_iter()
            .chain(readable_files)
            .collect::<Vec<_>>()
    };
    Ok(files.into_iter().take(10).collect())
}

fn score_person(name: &str, text: &str, lines: &[String]) -> usize {
    let mut score = 0usize;
    let lower = name.to_ascii_lowercase();
    for verb in PERSON_VERBS {
        if text
            .to_ascii_lowercase()
            .contains(&format!("{lower} {verb}"))
        {
            score += 2;
        }
    }
    for line in lines {
        let line_lower = line.to_ascii_lowercase();
        if line_lower.starts_with(&format!("{lower}:"))
            || line_lower.starts_with(&format!("> {lower}:"))
            || line_lower.contains(&format!("hey {lower}"))
            || line_lower.contains(&format!("thanks {lower}"))
            || line_lower.contains(&format!("hi {lower}"))
        {
            score += 1;
        }
    }
    score
}

fn score_project(name: &str, text: &str, lines: &[String]) -> usize {
    let mut score = 0usize;
    let lower = name.to_ascii_lowercase();
    let lowered_text = text.to_ascii_lowercase();
    for hint in PROJECT_HINTS {
        if lowered_text.contains(&format!("{lower} {hint}"))
            || lowered_text.contains(&format!("{hint} {lower}"))
        {
            score += 2;
        }
    }
    for line in lines {
        let line_lower = line.to_ascii_lowercase();
        if line_lower.contains(&format!("building {lower}"))
            || line_lower.contains(&format!("built {lower}"))
            || line_lower.contains(&format!("deploy {lower}"))
            || line_lower.contains(&format!("launch {lower}"))
            || line_lower.contains(&format!("{lower}.py"))
            || line_lower.contains(&format!("{lower}-core"))
        {
            score += 1;
        }
    }
    score
}

fn normalize_roomish(value: &str) -> String {
    value.to_ascii_lowercase().replace(['-', ' '], "_")
}

#[cfg(test)]
mod tests {
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
