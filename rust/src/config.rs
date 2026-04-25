//! Palace path and embedding configuration resolution.
//!
//! This module is the first storage boundary for the Rust rewrite: it decides
//! where the palace lives on disk and which embedding profile must be honored
//! by both SQLite metadata and LanceDB tables.

use std::path::{Path, PathBuf};

use crate::error::{MempalaceError, Result};

/// Resolved runtime configuration shared by CLI, MCP, and service layers.
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub palace_path: PathBuf,
    pub embedding: EmbeddingSettings,
}

/// Supported embedding backends for the current rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmbeddingBackend {
    Fastembed,
    Hash,
}

/// Embedding-specific configuration persisted alongside palace metadata.
#[derive(Clone, Debug)]
pub struct EmbeddingSettings {
    pub backend: EmbeddingBackend,
    pub model: String,
    pub cache_dir: PathBuf,
    pub hf_endpoint: Option<String>,
    pub show_download_progress: bool,
}

impl AppConfig {
    /// Resolves the palace path and embedding settings from explicit input,
    /// environment overrides, or the default home-directory location.
    pub fn resolve(explicit: Option<impl AsRef<Path>>) -> Result<Self> {
        let home = dirs::home_dir();

        if let Some(path) = explicit {
            let palace_path = normalize_path(path.as_ref())?;
            return Ok(Self {
                embedding: resolve_embedding_settings(home.as_ref(), &palace_path)?,
                palace_path,
            });
        }

        if let Ok(path) = std::env::var("MEMPALACE_RS_PALACE_PATH") {
            let palace_path = normalize_path(Path::new(&path))?;
            return Ok(Self {
                embedding: resolve_embedding_settings(home.as_ref(), &palace_path)?,
                palace_path,
            });
        }

        let home = home.ok_or_else(|| {
            MempalaceError::InvalidArgument("Unable to determine home directory".to_string())
        })?;
        let palace_path = home.join(".mempalace-rs").join("palace");

        Ok(Self {
            embedding: resolve_embedding_settings(Some(&home), &palace_path)?,
            palace_path,
        })
    }

    /// Returns the SQLite metadata path for the palace.
    pub fn sqlite_path(&self) -> PathBuf {
        self.palace_path.join("palace.sqlite3")
    }

    /// Returns the LanceDB directory used for semantic search vectors.
    pub fn lance_path(&self) -> PathBuf {
        self.palace_path.join("lance")
    }

    /// Returns the palace-local identity layer path.
    pub fn identity_path(&self) -> PathBuf {
        self.palace_path.join("identity.txt")
    }

    /// Ensures the palace root and vector-store directory exist before use.
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.palace_path)?;
        std::fs::create_dir_all(self.lance_path())?;
        Ok(())
    }
}

fn normalize_path(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(MempalaceError::InvalidArgument(
            "Palace path cannot be empty".to_string(),
        ));
    }

    let raw = path.to_string_lossy();
    let expanded = if raw == "~" || raw.starts_with("~/") {
        let home = dirs::home_dir().ok_or_else(|| {
            MempalaceError::InvalidArgument("Unable to determine home directory".to_string())
        })?;
        if raw == "~" {
            home
        } else {
            home.join(raw.trim_start_matches("~/"))
        }
    } else {
        path.to_path_buf()
    };

    if expanded.is_absolute() {
        return Ok(expanded);
    }

    Ok(std::env::current_dir()?.join(expanded))
}

fn resolve_embedding_settings(
    home: Option<&PathBuf>,
    palace_path: &Path,
) -> Result<EmbeddingSettings> {
    let backend = match std::env::var("MEMPALACE_RS_EMBED_PROVIDER")
        .unwrap_or_else(|_| "fastembed".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "fastembed" => EmbeddingBackend::Fastembed,
        "hash" => EmbeddingBackend::Hash,
        other => {
            return Err(MempalaceError::InvalidArgument(format!(
                "Unsupported embedding provider: {other}"
            )));
        }
    };

    let model = std::env::var("MEMPALACE_RS_EMBED_MODEL")
        .unwrap_or_else(|_| "MultilingualE5Small".to_string());
    let cache_dir = match std::env::var("MEMPALACE_RS_EMBED_CACHE_DIR") {
        Ok(path) => normalize_path(Path::new(&path))?,
        Err(_) => home
            .map(|home| home.join(".mempalace-rs").join("models"))
            .unwrap_or_else(|| palace_path.join("models")),
    };
    let hf_endpoint = std::env::var("MEMPALACE_RS_HF_ENDPOINT")
        .ok()
        .or_else(|| std::env::var("HF_ENDPOINT").ok());
    let show_download_progress = std::env::var("MEMPALACE_RS_EMBED_SHOW_DOWNLOAD_PROGRESS")
        .map(|value| !matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "no"))
        .unwrap_or(true);

    Ok(EmbeddingSettings {
        backend,
        model,
        cache_dir,
        hf_endpoint,
        show_download_progress,
    })
}
