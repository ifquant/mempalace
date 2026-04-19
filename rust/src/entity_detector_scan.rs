use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

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
