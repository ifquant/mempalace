//! Mutating `App` helpers for palace and knowledge-graph operations.
//!
//! These methods all ensure the palace exists before delegating into
//! `PalaceOpsRuntime`, which keeps write-side behavior separate from the
//! read-only surfaces.

use crate::error::Result;
use crate::model::{
    DiaryReadResult, DiaryWriteResult, DrawerDeleteResult, DrawerWriteResult, KgInvalidateResult,
    KgQueryResult, KgStats, KgTimelineResult, KgTriple, KgWriteResult,
};
use crate::palace_ops::PalaceOpsRuntime;
use crate::service::App;

impl App {
    /// Insert one raw KG triple after ensuring the palace storage exists.
    pub async fn add_kg_triple(&self, triple: &KgTriple) -> Result<()> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .add_kg_triple(triple)
        .await
    }

    /// Fetch raw KG triples for one subject.
    pub async fn query_kg(&self, subject: &str) -> Result<Vec<KgTriple>> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .query_kg_raw(subject)
        .await
    }

    /// Run the user-facing KG query surface with direction and time filters.
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

    /// Return a timeline-oriented KG view, optionally scoped to one entity.
    pub async fn kg_timeline(&self, entity: Option<&str>) -> Result<KgTimelineResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_timeline(entity)
        .await
    }

    /// Return aggregate KG counts used by CLI and MCP health surfaces.
    pub async fn kg_stats(&self) -> Result<KgStats> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .kg_stats()
        .await
    }

    /// Add one KG fact through the normalized write path.
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

    /// Close out an existing KG fact without deleting its history.
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

    /// File one drawer directly into the palace.
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

    /// Delete one drawer by storage identifier.
    pub async fn delete_drawer(&self, drawer_id: &str) -> Result<DrawerDeleteResult> {
        self.init().await?;
        PalaceOpsRuntime {
            config: &self.config,
            embedder: self.embedder.as_ref(),
        }
        .delete_drawer(drawer_id)
        .await
    }

    /// Append one diary entry for the named agent.
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

    /// Read the most recent diary entries for one agent.
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
