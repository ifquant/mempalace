use std::path::{Path, PathBuf};

use crate::error::{MempalaceError, Result};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub palace_path: PathBuf,
}

impl AppConfig {
    pub fn resolve(explicit: Option<impl AsRef<Path>>) -> Result<Self> {
        if let Some(path) = explicit {
            return Ok(Self {
                palace_path: normalize_path(path.as_ref())?,
            });
        }

        if let Ok(path) = std::env::var("MEMPALACE_RS_PALACE_PATH") {
            return Ok(Self {
                palace_path: normalize_path(Path::new(&path))?,
            });
        }

        let home = dirs::home_dir().ok_or_else(|| {
            MempalaceError::InvalidArgument("Unable to determine home directory".to_string())
        })?;

        Ok(Self {
            palace_path: home.join(".mempalace-rs").join("palace"),
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
