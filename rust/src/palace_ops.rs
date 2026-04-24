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

    pub async fn add_kg_triple(&self, triple: &KgTriple) -> Result<()> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).add_triple(triple).map(|_| ())
    }

    pub async fn query_kg_raw(&self, subject: &str) -> Result<Vec<KgTriple>> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).query_raw(subject)
    }

    pub async fn kg_query(
        &self,
        entity: &str,
        as_of: Option<&str>,
        direction: &str,
    ) -> Result<KgQueryResult> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).query_entity(entity, as_of, direction)
    }

    pub async fn kg_timeline(&self, entity: Option<&str>) -> Result<KgTimelineResult> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).timeline(entity)
    }

    pub async fn kg_stats(&self) -> Result<KgStats> {
        let sqlite = self.open_sqlite()?;
        KnowledgeGraph::new(&sqlite).stats()
    }

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
                let _ = vector
                    .delete_drawer(self.embedder.profile().dimension, &drawer.id)
                    .await;
                Err(err)
            }
        }
    }

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
                if let Ok(embedding) = self.embedder.embed_query(&drawer_input.text) {
                    let _ = vector.add_drawers(&[drawer_input], &[embedding]).await;
                }
                Err(err)
            }
        }
    }

    pub async fn diary_write(
        &self,
        agent_name: &str,
        entry: &str,
        topic: &str,
    ) -> Result<DiaryWriteResult> {
        let sqlite = self.open_sqlite()?;
        sqlite.add_diary_entry(agent_name, topic, entry)
    }

    pub async fn diary_read(&self, agent_name: &str, last_n: usize) -> Result<DiaryReadResult> {
        let sqlite = self.open_sqlite()?;
        sqlite.read_diary_entries(agent_name, last_n)
    }
}
