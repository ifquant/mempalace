//! Write-oriented palace runtime for manual drawers, diary entries, and KG ops.
//!
//! This file contains the mutating palace facade. The key audit concern here is
//! cross-store coordination: SQLite owns canonical metadata while LanceDB owns
//! search vectors, so mutating flows must leave both stores in a recoverable
//! state when one side fails.

use crate::config::AppConfig;
use crate::drawers::{build_manual_drawer, drawer_input_from_record, sanitize_name};
use crate::embed::EmbeddingProvider;
use crate::error::Result;
use crate::knowledge_graph::KnowledgeGraph;
use crate::model::{
    DiaryReadResult, DiaryWriteResult, DrawerDeleteResult, DrawerWriteResult, KgInvalidateResult,
    KgQueryResult, KgStats, KgTimelineResult, KgTriple, KgWriteResult,
};
use crate::storage::sqlite::SqliteStore;
use crate::storage::vector::VectorStore;

/// Write-capable palace facade shared by CLI, MCP, and integration tests.
pub struct PalaceOpsRuntime<'a> {
    pub config: &'a AppConfig,
    pub embedder: &'a dyn EmbeddingProvider,
}

impl<'a> PalaceOpsRuntime<'a> {
    fn open_sqlite(&self) -> Result<SqliteStore> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        Ok(sqlite)
    }

    /// Inserts a knowledge-graph triple after storage initialization.
    pub async fn add_kg_triple(&self, triple: &KgTriple) -> Result<()> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).add_triple(triple).map(|_| ())
    }

    /// Returns raw KG triples for a single subject without higher-level shaping.
    pub async fn query_kg_raw(&self, subject: &str) -> Result<Vec<KgTriple>> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).query_raw(subject)
    }

    /// Queries KG facts for an entity with optional temporal and direction filters.
    pub async fn kg_query(
        &self,
        entity: &str,
        as_of: Option<&str>,
        direction: &str,
    ) -> Result<KgQueryResult> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).query_entity(entity, as_of, direction)
    }

    /// Returns a chronological KG view for one entity or the full graph.
    pub async fn kg_timeline(&self, entity: Option<&str>) -> Result<KgTimelineResult> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).timeline(entity)
    }

    /// Returns aggregate KG statistics from SQLite.
    pub async fn kg_stats(&self) -> Result<KgStats> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).stats()
    }

    /// Adds a sanitized KG triple through the higher-level write surface.
    pub async fn kg_add(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        valid_from: Option<&str>,
    ) -> Result<KgWriteResult> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).add_triple(&KgTriple {
            subject: sanitize_name(subject, "subject")?,
            predicate: sanitize_name(predicate, "predicate")?,
            object: sanitize_name(object, "object")?,
            valid_from: valid_from.map(ToOwned::to_owned),
            valid_to: None,
        })
    }

    /// Closes an active KG triple by setting `valid_to`.
    pub async fn kg_invalidate(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        ended: Option<&str>,
    ) -> Result<KgInvalidateResult> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).invalidate(
            &sanitize_name(subject, "subject")?,
            &sanitize_name(predicate, "predicate")?,
            &sanitize_name(object, "object")?,
            ended,
        )
    }

    /// Adds a manual drawer and mirrors it into both SQLite and LanceDB.
    pub async fn add_drawer(
        &self,
        wing: &str,
        room: &str,
        content: &str,
        source_file: Option<&str>,
        added_by: Option<&str>,
    ) -> Result<DrawerWriteResult> {
        let drawer = build_manual_drawer(wing, room, content, source_file, added_by)?;

        let sqlite = self.open_sqlite()?;
        if sqlite.drawer_exists(&drawer.id)? {
            return Ok(DrawerWriteResult {
                success: true,
                drawer_id: drawer.id,
                wing: drawer.wing,
                room: drawer.room,
                reason: Some("already_exists".to_string()),
            });
        }

        let embedding = self.embedder.embed_query(&drawer.text)?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        vector
            .add_drawers(std::slice::from_ref(&drawer), &[embedding])
            .await?;
        match sqlite.insert_drawer(&drawer) {
            Ok(result) => Ok(result),
            Err(err) => {
                // SQLite is the canonical metadata store, so roll back the
                // vector insert if the second half of the write fails.
                let _ = vector
                    .delete_drawer(self.embedder.profile().dimension, &drawer.id)
                    .await;
                Err(err)
            }
        }
    }

    /// Deletes a drawer from both stores, attempting to restore LanceDB if the
    /// SQLite delete fails after the vector row has already been removed.
    pub async fn delete_drawer(&self, drawer_id: &str) -> Result<DrawerDeleteResult> {
        let sqlite = self.open_sqlite()?;
        let vector = VectorStore::connect(&self.config.lance_path()).await?;
        let drawer = sqlite.get_drawer(drawer_id)?;
        vector
            .delete_drawer(self.embedder.profile().dimension, drawer_id)
            .await?;
        match sqlite.delete_drawer(drawer_id) {
            Ok(result) => Ok(result),
            Err(err) => {
                let drawer_input = drawer_input_from_record(&drawer);
                // Best-effort restore keeps the two stores aligned when the
                // canonical SQLite delete rejects the operation mid-flight.
                if let Ok(embedding) = self.embedder.embed_query(&drawer_input.text) {
                    let _ = vector.add_drawers(&[drawer_input], &[embedding]).await;
                }
                Err(err)
            }
        }
    }

    /// Appends an agent diary entry into SQLite.
    pub async fn diary_write(
        &self,
        agent_name: &str,
        entry: &str,
        topic: &str,
    ) -> Result<DiaryWriteResult> {
        let sqlite = self.open_sqlite()?;
        sqlite.add_diary_entry(agent_name, topic, entry)
    }

    /// Reads the most recent diary entries for an agent.
    pub async fn diary_read(&self, agent_name: &str, last_n: usize) -> Result<DiaryReadResult> {
        let sqlite = self.open_sqlite()?;
        sqlite.read_diary_entries(agent_name, last_n)
    }
}
