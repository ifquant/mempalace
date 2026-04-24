use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::{RepairPruneSummary, RepairRebuildSummary, RepairScanSummary, RepairSummary};
use crate::storage::sqlite::DrawerRecord;
use crate::storage::vector::VectorDrawer;

pub struct RepairContext {
    pub palace_path: PathBuf,
    pub sqlite_path: PathBuf,
    pub lance_path: PathBuf,
    pub version: String,
}

pub struct RepairDiagnostics {
    pub sqlite_exists: bool,
    pub lance_exists: bool,
    pub schema_version: Option<i64>,
    pub sqlite_drawer_count: Option<usize>,
    pub embedding_provider: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dimension: Option<usize>,
    pub vector_accessible: bool,
    pub issues: Vec<String>,
}

impl RepairContext {
    pub fn corrupt_ids_path(&self) -> PathBuf {
        corrupt_ids_path(&self.palace_path)
    }

    pub fn build_summary(&self, diagnostics: RepairDiagnostics) -> RepairSummary {
        RepairSummary {
            kind: "repair".to_string(),
            palace_path: self.palace_path.display().to_string(),
            sqlite_path: self.sqlite_path.display().to_string(),
            lance_path: self.lance_path.display().to_string(),
            version: self.version.clone(),
            sqlite_exists: diagnostics.sqlite_exists,
            lance_exists: diagnostics.lance_exists,
            schema_version: diagnostics.schema_version,
            sqlite_drawer_count: diagnostics.sqlite_drawer_count,
            embedding_provider: diagnostics.embedding_provider,
            embedding_model: diagnostics.embedding_model,
            embedding_dimension: diagnostics.embedding_dimension,
            vector_accessible: diagnostics.vector_accessible,
            ok: diagnostics.issues.is_empty(),
            issues: diagnostics.issues,
        }
    }

    pub fn build_scan_summary(
        &self,
        wing: Option<&str>,
        sqlite_drawers: &[DrawerRecord],
        vector_drawers: &[VectorDrawer],
    ) -> io::Result<RepairScanSummary> {
        let sqlite_ids = sqlite_drawers
            .iter()
            .map(|drawer| drawer.id.clone())
            .collect::<BTreeSet<_>>();
        let vector_ids = vector_drawers
            .iter()
            .map(|drawer| drawer.id.clone())
            .collect::<BTreeSet<_>>();

        let missing_from_vector = sqlite_ids
            .difference(&vector_ids)
            .cloned()
            .collect::<Vec<_>>();
        let orphaned_in_vector = vector_ids
            .difference(&sqlite_ids)
            .cloned()
            .collect::<Vec<_>>();
        let prune_candidates = orphaned_in_vector.len();

        let corrupt_ids_path = self.corrupt_ids_path();
        let mut payload = String::new();
        for drawer_id in &orphaned_in_vector {
            payload.push_str(drawer_id);
            payload.push('\n');
        }
        fs::write(&corrupt_ids_path, payload)?;

        Ok(RepairScanSummary {
            kind: "repair_scan".to_string(),
            palace_path: self.palace_path.display().to_string(),
            sqlite_path: self.sqlite_path.display().to_string(),
            lance_path: self.lance_path.display().to_string(),
            version: self.version.clone(),
            wing: wing.map(ToOwned::to_owned),
            sqlite_drawers: sqlite_drawers.len(),
            vector_drawers: vector_drawers.len(),
            missing_from_vector,
            orphaned_in_vector,
            corrupt_ids_path: corrupt_ids_path.display().to_string(),
            prune_candidates,
        })
    }

    pub fn build_prune_preview(&self, queued_ids: &[String], confirm: bool) -> RepairPruneSummary {
        RepairPruneSummary {
            kind: "repair_prune".to_string(),
            palace_path: self.palace_path.display().to_string(),
            sqlite_path: self.sqlite_path.display().to_string(),
            lance_path: self.lance_path.display().to_string(),
            version: self.version.clone(),
            corrupt_ids_path: self.corrupt_ids_path().display().to_string(),
            queued: queued_ids.len(),
            confirm,
            deleted_from_vector: 0,
            deleted_from_sqlite: 0,
            failed: 0,
        }
    }

    pub fn build_prune_result(
        &self,
        queued_ids: &[String],
        confirm: bool,
        deleted_from_vector: usize,
        deleted_from_sqlite: usize,
        failed: usize,
    ) -> RepairPruneSummary {
        RepairPruneSummary {
            kind: "repair_prune".to_string(),
            palace_path: self.palace_path.display().to_string(),
            sqlite_path: self.sqlite_path.display().to_string(),
            lance_path: self.lance_path.display().to_string(),
            version: self.version.clone(),
            corrupt_ids_path: self.corrupt_ids_path().display().to_string(),
            queued: queued_ids.len(),
            confirm,
            deleted_from_vector,
            deleted_from_sqlite,
            failed,
        }
    }

    pub fn build_rebuild_summary(
        &self,
        drawers_found: usize,
        rebuilt: usize,
        backup_path: Option<String>,
    ) -> RepairRebuildSummary {
        RepairRebuildSummary {
            kind: "repair_rebuild".to_string(),
            palace_path: self.palace_path.display().to_string(),
            sqlite_path: self.sqlite_path.display().to_string(),
            lance_path: self.lance_path.display().to_string(),
            version: self.version.clone(),
            drawers_found,
            rebuilt,
            backup_path,
        }
    }
}

pub fn corrupt_ids_path(palace_path: &Path) -> PathBuf {
    palace_path.join("corrupt_ids.txt")
}

pub fn read_corrupt_ids(path: &Path) -> io::Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let queued_ids = fs::read_to_string(path)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    Ok(queued_ids)
}

pub fn backup_sqlite_source(sqlite_path: &Path) -> io::Result<Option<String>> {
    if !sqlite_path.exists() {
        return Ok(None);
    }

    let backup_path = sqlite_path.with_extension("sqlite3.backup");
    fs::copy(sqlite_path, &backup_path)?;
    Ok(Some(backup_path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_corrupt_ids_ignores_blank_lines() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("corrupt_ids.txt");
        fs::write(&path, "a\n\n b \n").unwrap();

        let queued = read_corrupt_ids(&path).unwrap();
        assert_eq!(queued, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn backup_sqlite_source_copies_existing_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let sqlite_path = tempdir.path().join("palace.sqlite3");
        fs::write(&sqlite_path, "sqlite-data").unwrap();

        let backup = backup_sqlite_source(&sqlite_path).unwrap().unwrap();
        let backup_path = PathBuf::from(backup);
        assert!(backup_path.exists());
        assert_eq!(fs::read_to_string(backup_path).unwrap(), "sqlite-data");
    }
}
