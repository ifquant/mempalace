use std::collections::BTreeMap;
use std::path::Path;
use std::time::UNIX_EPOCH;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::embed::EmbeddingProfile;
use crate::error::Result;
use crate::model::{DrawerInput, KgTriple, Rooms, Taxonomy};

pub const CURRENT_SCHEMA_VERSION: i64 = 3;

#[derive(Clone, Debug, PartialEq)]
pub struct IngestedFileState {
    pub content_hash: String,
    pub source_mtime: Option<f64>,
}

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn init_schema(&self) -> Result<()> {
        self.ensure_meta_table()?;

        let version = self.schema_version()?.unwrap_or_else(|| {
            if self.has_user_tables().unwrap_or(false) {
                1
            } else {
                0
            }
        });

        match version {
            0 => self.bootstrap_schema()?,
            1 => self.migrate_v1_to_v2()?,
            2 => self.migrate_v2_to_v3()?,
            CURRENT_SCHEMA_VERSION => {}
            other => {
                return Err(crate::error::MempalaceError::InvalidArgument(format!(
                    "Unsupported palace schema_version {other}; expected <= {CURRENT_SCHEMA_VERSION}"
                )));
            }
        }

        Ok(())
    }

    pub fn schema_version(&self) -> Result<Option<i64>> {
        let value = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value.and_then(|raw| raw.parse::<i64>().ok()))
    }

    fn ensure_meta_table(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    fn bootstrap_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS drawers (
                id TEXT PRIMARY KEY,
                wing TEXT NOT NULL,
                room TEXT NOT NULL,
                source_path TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                text TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_drawers_wing ON drawers(wing);
            CREATE INDEX IF NOT EXISTS idx_drawers_room ON drawers(room);
            CREATE INDEX IF NOT EXISTS idx_drawers_source_path ON drawers(source_path);

            CREATE TABLE IF NOT EXISTS ingested_files (
                source_path TEXT PRIMARY KEY,
                content_hash TEXT NOT NULL,
                source_mtime REAL,
                wing TEXT NOT NULL,
                room TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS kg_triples (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subject TEXT NOT NULL,
                predicate TEXT NOT NULL,
                object TEXT NOT NULL,
                valid_from TEXT,
                valid_to TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                note TEXT NOT NULL
            );
            "#,
        )?;

        self.set_meta("schema_version", &CURRENT_SCHEMA_VERSION.to_string())?;
        self.record_migration(
            CURRENT_SCHEMA_VERSION,
            "bootstrap fresh schema with migration tracking",
        )?;
        Ok(())
    }

    fn migrate_v1_to_v2(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                note TEXT NOT NULL
            );
            "#,
        )?;
        self.set_meta("schema_version", &CURRENT_SCHEMA_VERSION.to_string())?;
        self.record_migration(
            2,
            "add schema_migrations table and promote schema version metadata",
        )?;
        Ok(())
    }

    fn migrate_v2_to_v3(&self) -> Result<()> {
        self.conn.execute(
            "ALTER TABLE ingested_files ADD COLUMN source_mtime REAL",
            [],
        )?;
        self.set_meta("schema_version", &CURRENT_SCHEMA_VERSION.to_string())?;
        self.record_migration(
            CURRENT_SCHEMA_VERSION,
            "add source_mtime tracking for project re-mine parity",
        )?;
        Ok(())
    }

    fn record_migration(&self, version: i64, note: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_migrations(version, applied_at, note) VALUES(?1, ?2, ?3)",
            params![version, Utc::now().to_rfc3339(), note],
        )?;
        Ok(())
    }

    fn has_user_tables(&self) -> Result<bool> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('drawers', 'ingested_files', 'kg_triples')",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(count > 0)
    }

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

    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        let value = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(value)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES(?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn ensure_embedding_profile(&self, profile: &EmbeddingProfile) -> Result<()> {
        let stored_provider = self.meta("embedding_provider")?;
        let stored_model = self.meta("embedding_model")?;
        let stored_dimension = self
            .meta("embedding_dimension")?
            .and_then(|value| value.parse::<usize>().ok());

        if let (Some(provider), Some(model), Some(dimension)) =
            (stored_provider, stored_model, stored_dimension)
        {
            if provider == profile.provider
                && model == profile.model
                && dimension == profile.dimension
            {
                return Ok(());
            }

            return Err(crate::error::MempalaceError::InvalidArgument(format!(
                "Palace embedding profile mismatch: existing={provider}/{model}/{dimension}, requested={}/{}/{}",
                profile.provider, profile.model, profile.dimension
            )));
        }

        if self.total_drawers()? > 0 {
            let legacy = EmbeddingProfile::legacy_hash();
            if &legacy != profile {
                return Err(crate::error::MempalaceError::InvalidArgument(format!(
                    "Existing palace contains legacy hash embeddings. Re-open it with hash provider or create a new palace for {}/{}",
                    profile.provider, profile.model
                )));
            }
        }

        self.persist_embedding_profile(profile)
    }

    fn persist_embedding_profile(&self, profile: &EmbeddingProfile) -> Result<()> {
        self.set_meta("embedding_provider", &profile.provider)?;
        self.set_meta("embedding_model", &profile.model)?;
        self.set_meta("embedding_dimension", &profile.dimension.to_string())?;
        Ok(())
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
                "INSERT INTO drawers (id, wing, room, source_path, source_hash, chunk_index, text, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for drawer in drawers {
                stmt.execute(params![
                    drawer.id,
                    drawer.wing,
                    drawer.room,
                    drawer.source_path,
                    drawer.source_hash,
                    drawer.chunk_index,
                    drawer.text,
                    now,
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

    pub fn source_mtime(path: &Path) -> Option<f64> {
        let modified = path.metadata().ok()?.modified().ok()?;
        let duration = modified.duration_since(UNIX_EPOCH).ok()?;
        Some(duration.as_secs_f64())
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

    pub fn add_kg_triple(&self, triple: &KgTriple) -> Result<()> {
        self.conn.execute(
            "INSERT INTO kg_triples(subject, predicate, object, valid_from, valid_to, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                triple.subject,
                triple.predicate,
                triple.object,
                triple.valid_from,
                triple.valid_to,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn query_kg(&self, subject: &str) -> Result<Vec<KgTriple>> {
        let mut stmt = self.conn.prepare(
            "SELECT subject, predicate, object, valid_from, valid_to
             FROM kg_triples WHERE subject = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([subject], |row| {
            Ok(KgTriple {
                subject: row.get(0)?,
                predicate: row.get(1)?,
                object: row.get(2)?,
                valid_from: row.get(3)?,
                valid_to: row.get(4)?,
            })
        })?;

        let mut triples = Vec::new();
        for row in rows {
            triples.push(row?);
        }
        Ok(triples)
    }
}
