use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::error::Result;

use super::include::{is_exact_force_include, is_force_include, normalize_include_paths};

const CONVO_EXTENSIONS: &[&str] = &[".txt", ".md", ".json", ".jsonl"];
const MAX_CONVO_FILE_SIZE: u64 = 10 * 1024 * 1024;

pub fn scan_convo_files(
    dir: &Path,
    respect_gitignore: bool,
    include_ignored: &[String],
    skip_dirs: &[&str],
) -> Result<Vec<PathBuf>> {
    let skip_dirs = skip_dirs
        .iter()
        .map(|item| (*item).to_string())
        .collect::<HashSet<_>>();
    let include_paths = normalize_include_paths(include_ignored);
    let include_paths_for_filter = include_paths.clone();
    let project_root = dir.to_path_buf();
    let mut builder = WalkBuilder::new(dir);
    builder.hidden(false);
    builder.git_ignore(respect_gitignore);
    builder.git_global(respect_gitignore);
    builder.git_exclude(respect_gitignore);
    builder.require_git(false);
    builder.filter_entry(move |entry| {
        if is_force_include(entry.path(), &project_root, &include_paths_for_filter) {
            return true;
        }

        entry
            .file_name()
            .to_str()
            .map(|name| !skip_dirs.contains(name) && !name.ends_with(".egg-info"))
            .unwrap_or(true)
    });

    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for result in builder.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() || path.is_symlink() {
            continue;
        }

        let exact_force_include = is_exact_force_include(path, dir, &include_paths);
        if !exact_force_include && should_skip_convo_file(path) {
            continue;
        }

        let stat = match path.metadata() {
            Ok(stat) => stat,
            Err(_) => continue,
        };
        if stat.len() > MAX_CONVO_FILE_SIZE {
            continue;
        }

        if seen.insert(path.to_path_buf()) {
            files.push(path.to_path_buf());
        }
    }

    for rel in include_ignored {
        let path = dir.join(rel);
        if path.is_file() && seen.insert(path.clone()) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

pub(crate) fn should_skip_convo_file(path: &Path) -> bool {
    let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    if filename.ends_with(".meta.json") {
        return true;
    }
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext.to_ascii_lowercase()))
        .unwrap_or_default();
    !CONVO_EXTENSIONS.iter().any(|candidate| *candidate == ext)
}
