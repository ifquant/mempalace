use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use regex::Regex;

use crate::error::Result;

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

#[derive(Clone, Debug, PartialEq)]
pub struct DetectedEntities {
    pub people: Vec<String>,
    pub projects: Vec<String>,
    pub files_scanned: usize,
}

pub fn detect_entities(project_dir: &Path) -> Result<DetectedEntities> {
    let files = scan_for_detection(project_dir)?;
    if files.is_empty() {
        return Ok(DetectedEntities {
            people: Vec::new(),
            projects: Vec::new(),
            files_scanned: 0,
        });
    }

    let mut all_text = String::new();
    let mut all_lines = Vec::new();
    let max_bytes = 5_000usize;
    let files_scanned = files.len().min(10);

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
        files_scanned,
    })
}

pub fn detect_entities_for_registry(project_dir: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let detected = detect_entities(project_dir)?;
    Ok((detected.people, detected.projects))
}

pub fn scan_for_detection(project_dir: &Path) -> Result<Vec<PathBuf>> {
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
        if path.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|name| SKIP_DIRS.contains(&name))
        }) {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{detect_entities, scan_for_detection};

    #[test]
    fn entity_detector_prefers_prose_files_and_detects_people_projects() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("docs")).unwrap();
        fs::write(
            project.join("docs").join("notes.md"),
            "Jordan said the Atlas service should launch next week.\nJordan wrote the Atlas architecture guide.",
        )
        .unwrap();
        fs::write(
            project.join("README.txt"),
            "Jordan asked whether Atlas should deploy on Friday.\nJordan pushed the Atlas repo.",
        )
        .unwrap();

        let detected = detect_entities(&project).unwrap();
        assert_eq!(detected.files_scanned, 2);
        assert!(detected.people.iter().any(|name| name == "Jordan"));
        assert!(detected.projects.iter().any(|name| name == "Atlas"));
    }

    #[test]
    fn scan_for_detection_skips_noise_directories() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("target")).unwrap();
        fs::create_dir_all(project.join("notes")).unwrap();
        fs::write(
            project.join("target").join("generated.md"),
            "Jordan said Atlas.",
        )
        .unwrap();
        fs::write(project.join("notes").join("real.md"), "Jordan said Atlas.").unwrap();

        let files = scan_for_detection(&project).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("real.md"));
    }
}
