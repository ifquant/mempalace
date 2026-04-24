use crate::error::Result;
use crate::model::{
    DiaryReadResult, DiaryWriteResult, DrawerDeleteResult, DrawerWriteResult, KgInvalidateResult,
    KgQueryResult, KgStats, KgTimelineResult, KgTriple, KgWriteResult,
};
use crate::palace_ops::PalaceOpsRuntime;
use crate::service::App;

impl App {
    pub async fn add_kg_triple(&self, triple: &KgTriple) -> Result<()> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .add_kg_triple(triple)
        .await
    }

    pub async fn query_kg(&self, subject: &str) -> Result<Vec<KgTriple>> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .query_kg_raw(subject)
        .await
    }

    pub async fn kg_query(
        &self,
        entity: &str,
        as_of: Option<&str>,
        direction: &str,
    ) -> Result<KgQueryResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_query(entity, as_of, direction)
        .await
    }

    pub async fn kg_timeline(&self, entity: Option<&str>) -> Result<KgTimelineResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_timeline(entity)
        .await
    }

    pub async fn kg_stats(&self) -> Result<KgStats> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_stats()
        .await
    }

    pub async fn kg_add(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        valid_from: Option<&str>,
    ) -> Result<KgWriteResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_add(subject, predicate, object, valid_from)
        .await
    }

    pub async fn kg_invalidate(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        ended: Option<&str>,
    ) -> Result<KgInvalidateResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_invalidate(subject, predicate, object, ended)
        .await
    }

    pub async fn add_drawer(
        &self,
        wing: &str,
        room: &str,
        content: &str,
        source_file: Option<&str>,
        added_by: Option<&str>,
    ) -> Result<DrawerWriteResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .add_drawer(wing, room, content, source_file, added_by)
        .await
    }

    pub async fn delete_drawer(&self, drawer_id: &str) -> Result<DrawerDeleteResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .delete_drawer(drawer_id)
        .await
    }

    pub async fn diary_write(
        &self,
        agent_name: &str,
        entry: &str,
        topic: &str,
    ) -> Result<DiaryWriteResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .diary_write(agent_name, entry, topic)
        .await
    }

    pub async fn diary_read(&self, agent_name: &str, last_n: usize) -> Result<DiaryReadResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .diary_read(agent_name, last_n)
        .await
    }
}
