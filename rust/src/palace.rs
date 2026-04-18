use std::path::Path;

use crate::config::AppConfig;
use crate::embed::EmbeddingProfile;
use crate::error::Result;
use crate::storage::sqlite::SqliteStore;
use crate::storage::vector::VectorStore;

pub const SKIP_DIRS: &[&str] = &[
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

pub async fn ensure_vector_store(
    config: &AppConfig,
    profile: &EmbeddingProfile,
) -> Result<VectorStore> {
    config.ensure_dirs()?;
    let vector = VectorStore::connect(&config.lance_path()).await?;
    let _ = vector.ensure_table(profile.dimension).await?;
    Ok(vector)
}

pub fn file_already_mined(
    sqlite: &SqliteStore,
    source_path: &Path,
    check_mtime: bool,
) -> Result<bool> {
    let source_path_text = source_path.display().to_string();
    let Some(state) = sqlite.ingested_file_state(&source_path_text)? else {
        return Ok(false);
    };

    if !check_mtime {
        return Ok(true);
    }

    let Some(stored_mtime) = state.source_mtime else {
        return Ok(false);
    };
    let Some(current_mtime) = SqliteStore::source_mtime(source_path) else {
        return Ok(false);
    };
    Ok(stored_mtime == current_mtime)
}

pub fn source_state_matches(
    sqlite: &SqliteStore,
    source_path: &Path,
    content_hash: &str,
    source_mtime: Option<f64>,
    check_mtime: bool,
) -> Result<bool> {
    let source_path_text = source_path.display().to_string();
    let Some(state) = sqlite.ingested_file_state(&source_path_text)? else {
        return Ok(false);
    };

    if check_mtime
        && let Some(stored_mtime) = state.source_mtime
        && let Some(current_mtime) = source_mtime
    {
        return Ok(stored_mtime == current_mtime);
    }

    Ok(state.content_hash == content_hash)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{file_already_mined, source_state_matches};
    use crate::storage::sqlite::SqliteStore;

    #[test]
    fn file_already_mined_respects_check_mtime() {
        let tmp = tempdir().unwrap();
        let mut sqlite = SqliteStore::open(&tmp.path().join("palace.sqlite3")).unwrap();
        sqlite.init_schema().unwrap();

        let source = tmp.path().join("note.txt");
        fs::write(&source, "hello").unwrap();
        let source_mtime = SqliteStore::source_mtime(&source);
        sqlite
            .replace_source(
                &source.display().to_string(),
                "project",
                "general",
                "hash-1",
                source_mtime,
                &[],
            )
            .unwrap();

        assert!(file_already_mined(&sqlite, &source, false).unwrap());
        assert!(file_already_mined(&sqlite, &source, true).unwrap());
    }

    #[test]
    fn source_state_matches_uses_hash_when_mtime_check_cannot_match() {
        let tmp = tempdir().unwrap();
        let mut sqlite = SqliteStore::open(&tmp.path().join("palace.sqlite3")).unwrap();
        sqlite.init_schema().unwrap();

        let source = tmp.path().join("note.txt");
        fs::write(&source, "hello").unwrap();
        sqlite
            .replace_source(
                &source.display().to_string(),
                "project",
                "general",
                "same-hash",
                None,
                &[],
            )
            .unwrap();

        assert!(source_state_matches(&sqlite, &source, "same-hash", None, true).unwrap());
        assert!(!source_state_matches(&sqlite, &source, "other-hash", None, true).unwrap());
    }
}
