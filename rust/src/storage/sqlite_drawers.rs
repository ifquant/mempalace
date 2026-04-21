use std::collections::BTreeMap;

use chrono::Utc;
use rusqlite::{OptionalExtension, params};

use crate::error::{MempalaceError, Result};
use crate::model::{
    CompressedDrawer, DrawerDeleteResult, DrawerInput, DrawerWriteResult, Rooms, Taxonomy,
};

use super::{DrawerRecord, GraphRoomRow, IngestedFileState, SqliteStore};

impl SqliteStore {
    pub fn ingested_file_state(&self, source_path: &str) -> Result<Option<IngestedFileState>> {
        let value = self
            .conn
            .query_row(
                "SELECT content_hash, source_mtime FROM ingested_files WHERE source_path = ?1",
                [source_path],
                |row| {
                    Ok(IngestedFileState {
                        content_hash: row.get(0)?,
                        source_mtime: row.get(1)?,
                    })
                },
            )
            .optional()?;
        Ok(value)
    }

    pub fn replace_source(
        &mut self,
        source_path: &str,
        wing: &str,
        room: &str,
        content_hash: &str,
        source_mtime: Option<f64>,
        drawers: &[DrawerInput],
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM drawers WHERE source_path = ?1", [source_path])?;

        let now = Utc::now().to_rfc3339();
        {
            let mut stmt = tx.prepare(
                "INSERT INTO drawers (id, wing, room, source_file, source_path, source_hash, source_mtime, chunk_index, added_by, filed_at, ingest_mode, extract_mode, text, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            )?;
            for drawer in drawers {
                stmt.execute(params![
                    &drawer.id,
                    &drawer.wing,
                    &drawer.room,
                    &drawer.source_file,
                    &drawer.source_path,
                    &drawer.source_hash,
                    drawer.source_mtime,
                    drawer.chunk_index,
                    &drawer.added_by,
                    &drawer.filed_at,
                    &drawer.ingest_mode,
                    &drawer.extract_mode,
                    &drawer.text,
                    &now,
                ])?;
            }
        }

        tx.execute(
            "INSERT OR REPLACE INTO ingested_files (source_path, content_hash, source_mtime, wing, room, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![source_path, content_hash, source_mtime, wing, room, now],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn drawer_exists(&self, drawer_id: &str) -> Result<bool> {
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM drawers WHERE id = ?1 LIMIT 1",
                [drawer_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        Ok(exists.is_some())
    }

    pub fn get_drawer(&self, drawer_id: &str) -> Result<DrawerRecord> {
        let drawer = self
            .conn
            .query_row(
                "SELECT id, wing, room, source_file, source_path, source_hash, source_mtime, chunk_index, added_by, filed_at, ingest_mode, extract_mode, text
                 FROM drawers
                 WHERE id = ?1",
                [drawer_id],
                map_drawer_record,
            )
            .optional()?;
        drawer.ok_or_else(|| {
            MempalaceError::InvalidArgument(format!("Drawer not found: {drawer_id}"))
        })
    }

    pub fn insert_drawer(&self, drawer: &DrawerInput) -> Result<DrawerWriteResult> {
        if self.drawer_exists(&drawer.id)? {
            return Ok(DrawerWriteResult {
                success: true,
                drawer_id: drawer.id.clone(),
                wing: drawer.wing.clone(),
                room: drawer.room.clone(),
                reason: Some("already_exists".to_string()),
            });
        }

        self.conn.execute(
            "INSERT INTO drawers (id, wing, room, source_file, source_path, source_hash, source_mtime, chunk_index, added_by, filed_at, ingest_mode, extract_mode, text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                &drawer.id,
                &drawer.wing,
                &drawer.room,
                &drawer.source_file,
                &drawer.source_path,
                &drawer.source_hash,
                drawer.source_mtime,
                drawer.chunk_index,
                &drawer.added_by,
                &drawer.filed_at,
                &drawer.ingest_mode,
                &drawer.extract_mode,
                &drawer.text,
                Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(DrawerWriteResult {
            success: true,
            drawer_id: drawer.id.clone(),
            wing: drawer.wing.clone(),
            room: drawer.room.clone(),
            reason: None,
        })
    }

    pub fn delete_drawer(&self, drawer_id: &str) -> Result<DrawerDeleteResult> {
        let deleted = self
            .conn
            .execute("DELETE FROM drawers WHERE id = ?1", [drawer_id])?;
        if deleted == 0 {
            return Err(MempalaceError::InvalidArgument(format!(
                "Drawer not found: {drawer_id}"
            )));
        }
        Ok(DrawerDeleteResult {
            success: true,
            drawer_id: drawer_id.to_string(),
        })
    }

    pub fn delete_drawers(&self, drawer_ids: &[String]) -> Result<usize> {
        let mut deleted = 0usize;
        let mut stmt = self.conn.prepare("DELETE FROM drawers WHERE id = ?1")?;
        for drawer_id in drawer_ids {
            deleted += stmt.execute([drawer_id])?;
        }
        Ok(deleted)
    }

    pub fn total_drawers(&self) -> Result<usize> {
        let count = self
            .conn
            .query_row("SELECT COUNT(*) FROM drawers", [], |row| {
                row.get::<_, i64>(0)
            })?;
        Ok(count as usize)
    }

    pub fn list_wings(&self) -> Result<BTreeMap<String, usize>> {
        let mut stmt = self
            .conn
            .prepare("SELECT wing, COUNT(*) FROM drawers GROUP BY wing ORDER BY wing")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?;

        let mut out = BTreeMap::new();
        for row in rows {
            let (wing, count) = row?;
            out.insert(wing, count);
        }
        Ok(out)
    }

    pub fn list_rooms(&self, wing: Option<&str>) -> Result<Rooms> {
        let mut out = BTreeMap::new();
        if let Some(wing_name) = wing {
            let mut stmt = self.conn.prepare(
                "SELECT room, COUNT(*) FROM drawers WHERE wing = ?1 GROUP BY room ORDER BY room",
            )?;
            let rows = stmt.query_map([wing_name], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })?;
            for row in rows {
                let (room, count) = row?;
                out.insert(room, count);
            }
            return Ok(Rooms {
                wing: wing_name.to_string(),
                rooms: out,
            });
        }

        let mut stmt = self
            .conn
            .prepare("SELECT room, COUNT(*) FROM drawers GROUP BY room ORDER BY room")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?;
        for row in rows {
            let (room, count) = row?;
            out.insert(room, count);
        }
        Ok(Rooms {
            wing: "all".to_string(),
            rooms: out,
        })
    }

    pub fn taxonomy(&self) -> Result<Taxonomy> {
        let mut stmt = self.conn.prepare(
            "SELECT wing, room, COUNT(*) FROM drawers GROUP BY wing, room ORDER BY wing, room",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? as usize,
            ))
        })?;

        let mut taxonomy = BTreeMap::new();
        for row in rows {
            let (wing, room, count) = row?;
            taxonomy
                .entry(wing)
                .or_insert_with(BTreeMap::new)
                .insert(room, count);
        }
        Ok(Taxonomy { taxonomy })
    }

    pub fn list_drawers(&self, wing: Option<&str>) -> Result<Vec<DrawerRecord>> {
        let mut query = String::from(
            "SELECT id, wing, room, source_file, source_path, source_hash, source_mtime, chunk_index, added_by, filed_at, ingest_mode, extract_mode, text
             FROM drawers",
        );
        let mut records = Vec::new();
        if let Some(wing_name) = wing {
            query.push_str(
                " WHERE wing = ?1
                  ORDER BY wing, room, source_file, chunk_index, filed_at",
            );
            let mut stmt = self.conn.prepare(&query)?;
            let rows = stmt.query_map([wing_name], map_drawer_record)?;
            for row in rows {
                records.push(row?);
            }
        } else {
            query.push_str(" ORDER BY wing, room, source_file, chunk_index, filed_at");
            let mut stmt = self.conn.prepare(&query)?;
            let rows = stmt.query_map([], map_drawer_record)?;
            for row in rows {
                records.push(row?);
            }
        }
        Ok(records)
    }

    pub fn recent_drawers(&self, wing: Option<&str>, limit: usize) -> Result<Vec<DrawerRecord>> {
        let fetch_limit = limit.max(1) as i64;
        let mut records = Vec::new();
        if let Some(wing_name) = wing {
            let mut stmt = self.conn.prepare(
                "SELECT id, wing, room, source_file, source_path, source_hash, source_mtime, chunk_index, added_by, filed_at, ingest_mode, extract_mode, text
                 FROM drawers
                 WHERE wing = ?1
                 ORDER BY filed_at DESC, chunk_index ASC
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![wing_name, fetch_limit], map_drawer_record)?;
            for row in rows {
                records.push(row?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, wing, room, source_file, source_path, source_hash, source_mtime, chunk_index, added_by, filed_at, ingest_mode, extract_mode, text
                 FROM drawers
                 ORDER BY filed_at DESC, chunk_index ASC
                 LIMIT ?1",
            )?;
            let rows = stmt.query_map([fetch_limit], map_drawer_record)?;
            for row in rows {
                records.push(row?);
            }
        }
        Ok(records)
    }

    pub fn replace_compressed_drawers(
        &mut self,
        wing: Option<&str>,
        entries: &[CompressedDrawer],
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        if let Some(wing_name) = wing {
            tx.execute(
                "DELETE FROM compressed_drawers WHERE wing = ?1",
                [wing_name],
            )?;
        } else {
            tx.execute("DELETE FROM compressed_drawers", [])?;
        }
        let mut stmt = tx.prepare(
            "INSERT INTO compressed_drawers (drawer_id, wing, room, source_file, source_path, ingest_mode, extract_mode, aaak, original_tokens, compressed_tokens, compression_ratio, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )?;
        let now = Utc::now().to_rfc3339();
        for entry in entries {
            stmt.execute(params![
                &entry.drawer_id,
                &entry.wing,
                &entry.room,
                &entry.source_file,
                &entry.source_path,
                &entry.ingest_mode,
                &entry.extract_mode,
                &entry.aaak,
                entry.original_tokens as i64,
                entry.compressed_tokens as i64,
                entry.compression_ratio,
                &now,
            ])?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(())
    }

    pub fn list_compressed_drawers(&self, wing: Option<&str>) -> Result<Vec<CompressedDrawer>> {
        let mut query = String::from(
            "SELECT drawer_id, wing, room, source_file, source_path, ingest_mode, extract_mode, aaak, original_tokens, compressed_tokens, compression_ratio
             FROM compressed_drawers",
        );
        let mut entries = Vec::new();
        if let Some(wing_name) = wing {
            query.push_str(" WHERE wing = ?1 ORDER BY wing, room, source_file, drawer_id");
            let mut stmt = self.conn.prepare(&query)?;
            let rows = stmt.query_map([wing_name], map_compressed_drawer)?;
            for row in rows {
                entries.push(row?);
            }
        } else {
            query.push_str(" ORDER BY wing, room, source_file, drawer_id");
            let mut stmt = self.conn.prepare(&query)?;
            let rows = stmt.query_map([], map_compressed_drawer)?;
            for row in rows {
                entries.push(row?);
            }
        }
        Ok(entries)
    }

    pub fn graph_room_rows(&self) -> Result<Vec<GraphRoomRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT room, wing, filed_at
             FROM drawers
             WHERE room IS NOT NULL AND room != '' AND room != 'general' AND wing IS NOT NULL AND wing != ''
             ORDER BY room, wing, filed_at",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(GraphRoomRow {
                room: row.get(0)?,
                wing: row.get(1)?,
                filed_at: row.get(2)?,
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

fn map_drawer_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<DrawerRecord> {
    Ok(DrawerRecord {
        id: row.get(0)?,
        wing: row.get(1)?,
        room: row.get(2)?,
        source_file: row.get(3)?,
        source_path: row.get(4)?,
        source_hash: row.get(5)?,
        source_mtime: row.get(6)?,
        chunk_index: row.get(7)?,
        added_by: row.get(8)?,
        filed_at: row.get(9)?,
        ingest_mode: row.get(10)?,
        extract_mode: row.get(11)?,
        text: row.get(12)?,
    })
}

fn map_compressed_drawer(row: &rusqlite::Row<'_>) -> rusqlite::Result<CompressedDrawer> {
    Ok(CompressedDrawer {
        drawer_id: row.get(0)?,
        wing: row.get(1)?,
        room: row.get(2)?,
        source_file: row.get(3)?,
        source_path: row.get(4)?,
        ingest_mode: row.get(5)?,
        extract_mode: row.get(6)?,
        aaak: row.get(7)?,
        original_tokens: row.get::<_, i64>(8)? as usize,
        compressed_tokens: row.get::<_, i64>(9)? as usize,
        compression_ratio: row.get(10)?,
    })
}
