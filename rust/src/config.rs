use std::path::{Path, PathBuf};

use crate::error::{MempalaceError, Result};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub palace_path: PathBuf,
    pub embedding: EmbeddingSettings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmbeddingBackend {
    Fastembed,
    Hash,
}

#[derive(Clone, Debug)]
pub struct EmbeddingSettings {
    pub backend: EmbeddingBackend,
    pub model: String,
    pub cache_dir: PathBuf,
    pub show_download_progress: bool,
}

impl AppConfig {
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

    pub fn sqlite_path(&self) -> PathBuf {
        self.palace_path.join("palace.sqlite3")
    }

    pub fn lance_path(&self) -> PathBuf {
        self.palace_path.join("lance")
    }

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

    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    Ok(std::env::current_dir()?.join(path))
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
    let show_download_progress = std::env::var("MEMPALACE_RS_EMBED_SHOW_DOWNLOAD_PROGRESS")
        .map(|value| !matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "no"))
        .unwrap_or(true);

    Ok(EmbeddingSettings {
        backend,
        model,
        cache_dir,
        show_download_progress,
    })
}
