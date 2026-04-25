//! SQLite schema bootstrap and migration logic.
//!
//! This module is one of the main audit anchors for on-disk compatibility. It
//! upgrades older palace files in place and backfills newly added metadata so
//! higher layers can assume the modern schema contract.

use chrono::Utc;
use rusqlite::OptionalExtension;

use crate::embed::EmbeddingProfile;
use crate::error::{MempalaceError, Result};

use super::{CURRENT_SCHEMA_VERSION, SqliteStore};

impl SqliteStore {
    /// Bootstraps a fresh schema or migrates an existing palace up to the current version.
    pub fn init_schema(&self) -> Result<()> {
        self.ensure_meta_table()?;

        let mut version = self.schema_version()?.unwrap_or_else(|| {
            // Older palaces may predate explicit schema_version metadata, so
            // treat any detected user tables as the legacy v1 shape.
            if self.has_user_tables().unwrap_or(false) {
                1
            } else {
                0
            }
        });

        loop {
            match version {
                0 => {
                    self.bootstrap_schema()?;
                    break;
                }
                1 => {
                    self.migrate_v1_to_v2()?;
                    version = 2;
                }
                2 => {
                    self.migrate_v2_to_v3()?;
                    version = 3;
                }
                3 => {
                    self.migrate_v3_to_v4()?;
                    version = 4;
                }
                4 => {
                    self.migrate_v4_to_v5()?;
                    version = 5;
                }
                5 => {
                    self.migrate_v5_to_v6()?;
                    version = 6;
                }
                6 => {
                    self.migrate_v6_to_v7()?;
                    version = 7;
                }
                7 => {
                    self.migrate_v7_to_v8()?;
                    version = 8;
                }
                8 => {
                    self.migrate_v8_to_v9()?;
                    version = 9;
                }
                CURRENT_SCHEMA_VERSION => break,
                other => {
                    return Err(MempalaceError::InvalidArgument(format!(
                        "Unsupported palace schema_version {other}; expected <= {CURRENT_SCHEMA_VERSION}"
                    )));
                }
            }
        }

        Ok(())
    }

    /// Reads the recorded schema version from the meta table.
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

    /// Reads an arbitrary metadata value from the SQLite meta table.
    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        let value = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(value)
    }

    /// Writes an arbitrary metadata value into the SQLite meta table.
    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES(?1, ?2)",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    /// Verifies that the current palace matches the requested embedding profile.
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

            return Err(MempalaceError::InvalidArgument(format!(
                "Palace embedding profile mismatch: existing={provider}/{model}/{dimension}, requested={}/{}/{}",
                profile.provider, profile.model, profile.dimension
            )));
        }

        if self.total_drawers()? > 0 {
            let legacy = EmbeddingProfile::legacy_hash();
            if &legacy != profile {
                return Err(MempalaceError::InvalidArgument(format!(
                    "Existing palace contains legacy hash embeddings. Re-open it with hash provider or create a new palace for {}/{}",
                    profile.provider, profile.model
                )));
            }
        }

        self.persist_embedding_profile(profile)
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
                source_file TEXT NOT NULL,
                source_path TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                source_mtime REAL,
                chunk_index INTEGER NOT NULL,
                added_by TEXT NOT NULL,
                filed_at TEXT NOT NULL,
                ingest_mode TEXT NOT NULL,
                extract_mode TEXT NOT NULL,
                importance REAL,
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

            CREATE TABLE IF NOT EXISTS kg_entities (
                entity_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                note TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS diary_entries (
                id TEXT PRIMARY KEY,
                agent_name TEXT NOT NULL,
                wing TEXT NOT NULL,
                room TEXT NOT NULL,
                topic TEXT NOT NULL,
                entry TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                date TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS compressed_drawers (
                drawer_id TEXT PRIMARY KEY,
                wing TEXT NOT NULL,
                room TEXT NOT NULL,
                source_file TEXT NOT NULL,
                source_path TEXT NOT NULL,
                ingest_mode TEXT NOT NULL,
                extract_mode TEXT NOT NULL,
                aaak TEXT NOT NULL,
                original_tokens INTEGER NOT NULL,
                compressed_tokens INTEGER NOT NULL,
                compression_ratio REAL NOT NULL,
                created_at TEXT NOT NULL
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
        self.set_meta("schema_version", "2")?;
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
        self.set_meta("schema_version", "3")?;
        self.record_migration(3, "add source_mtime tracking for project re-mine parity")?;
        Ok(())
    }

    fn migrate_v3_to_v4(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            ALTER TABLE drawers RENAME TO drawers_v3;

            CREATE TABLE drawers (
                id TEXT PRIMARY KEY,
                wing TEXT NOT NULL,
                room TEXT NOT NULL,
                source_file TEXT NOT NULL,
                source_path TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                source_mtime REAL,
                chunk_index INTEGER NOT NULL,
                added_by TEXT NOT NULL,
                filed_at TEXT NOT NULL,
                text TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            INSERT INTO drawers (
                id,
                wing,
                room,
                source_file,
                source_path,
                source_hash,
                source_mtime,
                chunk_index,
                added_by,
                filed_at,
                text,
                created_at
            )
            SELECT
                id,
                wing,
                room,
                -- v3 stored only source_path, so backfill both source fields
                -- from that legacy value before dropping the old table.
                source_path,
                source_path,
                source_hash,
                NULL,
                chunk_index,
                'mempalace',
                created_at,
                text,
                created_at
            FROM drawers_v3;

            DROP TABLE drawers_v3;

            CREATE INDEX IF NOT EXISTS idx_drawers_wing ON drawers(wing);
            CREATE INDEX IF NOT EXISTS idx_drawers_room ON drawers(room);
            CREATE INDEX IF NOT EXISTS idx_drawers_source_path ON drawers(source_path);
            "#,
        )?;
        self.set_meta("schema_version", "4")?;
        self.record_migration(
            4,
            "add python-style drawer metadata fields: source_file, source_mtime, added_by, filed_at",
        )?;
        Ok(())
    }

    fn migrate_v4_to_v5(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS diary_entries (
                id TEXT PRIMARY KEY,
                agent_name TEXT NOT NULL,
                wing TEXT NOT NULL,
                room TEXT NOT NULL,
                topic TEXT NOT NULL,
                entry TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                date TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )?;
        self.set_meta("schema_version", "5")?;
        self.record_migration(5, "add agent diary entries table")?;
        Ok(())
    }

    fn migrate_v5_to_v6(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            ALTER TABLE drawers ADD COLUMN ingest_mode TEXT NOT NULL DEFAULT 'projects';
            ALTER TABLE drawers ADD COLUMN extract_mode TEXT NOT NULL DEFAULT 'exchange';
            "#,
        )?;
        self.set_meta("schema_version", "6")?;
        self.record_migration(
            6,
            "add ingest_mode and extract_mode drawer metadata for conversation mining parity",
        )?;
        Ok(())
    }

    fn migrate_v6_to_v7(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS compressed_drawers (
                drawer_id TEXT PRIMARY KEY,
                wing TEXT NOT NULL,
                room TEXT NOT NULL,
                source_file TEXT NOT NULL,
                source_path TEXT NOT NULL,
                ingest_mode TEXT NOT NULL,
                extract_mode TEXT NOT NULL,
                aaak TEXT NOT NULL,
                original_tokens INTEGER NOT NULL,
                compressed_tokens INTEGER NOT NULL,
                compression_ratio REAL NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )?;
        self.set_meta("schema_version", "7")?;
        self.record_migration(7, "add compressed_drawers table for AAAK summaries")?;
        Ok(())
    }

    fn migrate_v7_to_v8(&self) -> Result<()> {
        self.conn
            .execute("ALTER TABLE drawers ADD COLUMN importance REAL", [])?;
        self.set_meta("schema_version", "8")?;
        self.record_migration(8, "add canonical drawer importance metadata")?;
        Ok(())
    }

    fn migrate_v8_to_v9(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS kg_entities (
                entity_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )?;
        self.conn.execute_batch(
            r#"
            INSERT INTO kg_entities(entity_id, name, entity_type, created_at, updated_at)
            SELECT
                lower(replace(replace(entity, ' ', '_'), '-', '_')),
                entity,
                'unknown',
                CURRENT_TIMESTAMP,
                CURRENT_TIMESTAMP
            FROM (
                SELECT subject AS entity FROM kg_triples
                UNION
                SELECT object AS entity FROM kg_triples
            )
            WHERE trim(entity) <> ''
            ON CONFLICT(entity_id) DO NOTHING;
            "#,
        )?;
        // Existing triples become the seed set for explicit entity rows so the
        // KG can move forward with normalized entity metadata.
        self.set_meta("schema_version", "9")?;
        self.record_migration(9, "add explicit kg entity table for python parity")?;
        Ok(())
    }

    fn record_migration(&self, version: i64, note: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_migrations(version, applied_at, note) VALUES(?1, ?2, ?3)",
            rusqlite::params![version, Utc::now().to_rfc3339(), note],
        )?;
        Ok(())
    }

    fn has_user_tables(&self) -> Result<bool> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('drawers', 'ingested_files', 'kg_triples', 'kg_entities')",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(count > 0)
    }

    fn persist_embedding_profile(&self, profile: &EmbeddingProfile) -> Result<()> {
        self.set_meta("embedding_provider", &profile.provider)?;
        self.set_meta("embedding_model", &profile.model)?;
        self.set_meta("embedding_dimension", &profile.dimension.to_string())?;
        Ok(())
    }
}
