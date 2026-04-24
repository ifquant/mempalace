use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json::{Value, json};

use crate::error::Result;

pub struct WriteAheadLog {
    dir: PathBuf,
    file: PathBuf,
}

impl WriteAheadLog {
    pub fn for_palace(palace_path: &Path) -> Result<Self> {
        let dir = palace_path.join("wal");
        fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
        }

        let file = dir.join("write_log.jsonl");
        Ok(Self { dir, file })
    }

    pub fn log(&self, operation: &str, params: Value, result: Option<Value>) -> Result<()> {
        if !self.dir.exists() {
            fs::create_dir_all(&self.dir)?;
        }

        let entry = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "operation": operation,
            "params": params,
            "result": result,
        });

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)?;
        writeln!(file, "{}", serde_json::to_string(&entry)?)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.file, fs::Permissions::from_mode(0o600));
        }

        Ok(())
    }

    pub fn file_path(&self) -> &Path {
        &self.file
    }
}
