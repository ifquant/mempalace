//! Thin facade over the SQLite-backed knowledge graph.
//!
//! Audit readers should treat this as the high-level KG boundary: it exposes
//! entity/triple operations while the actual normalized-ID storage details stay
//! in `storage/sqlite_kg.rs`.

use crate::error::Result;
use crate::model::{
    KgEntityWriteResult, KgInvalidateResult, KgQueryResult, KgStats, KgTimelineResult, KgTriple,
    KgWriteResult,
};
use crate::storage::sqlite::SqliteStore;

pub struct KnowledgeGraph<'a> {
    store: &'a SqliteStore,
}

impl<'a> KnowledgeGraph<'a> {
    /// Binds KG operations to the canonical SQLite store.
    pub fn new(store: &'a SqliteStore) -> Self {
        Self { store }
    }

    /// Adds one fact triple, creating backing entity rows as needed.
    pub fn add_triple(&self, triple: &KgTriple) -> Result<KgWriteResult> {
        self.store.add_kg_triple(triple)
    }

    /// Upserts one entity into the KG entity table.
    pub fn add_entity(&self, name: &str, entity_type: &str) -> Result<KgEntityWriteResult> {
        // SQLite normalizes the durable entity ID internally so callers can keep
        // using human-readable names while the store maintains stable keys.
        self.store.add_kg_entity(name, entity_type)
    }

    /// Marks an active fact as no longer valid from `ended` onward.
    pub fn invalidate(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        ended: Option<&str>,
    ) -> Result<KgInvalidateResult> {
        self.store
            .invalidate_kg_triple(subject, predicate, object, ended)
    }

    /// Returns raw triples for one subject without direction/as-of filtering.
    pub fn query_raw(&self, subject: &str) -> Result<Vec<KgTriple>> {
        self.store.query_kg(subject)
    }

    /// Queries one entity with optional temporal and direction filters.
    pub fn query_entity(
        &self,
        entity: &str,
        as_of: Option<&str>,
        direction: &str,
    ) -> Result<KgQueryResult> {
        let facts = self.store.query_kg_entity(entity, as_of, direction)?;
        Ok(KgQueryResult {
            entity: entity.to_string(),
            as_of: as_of.map(ToOwned::to_owned),
            count: facts.len(),
            facts,
        })
    }

    /// Returns the KG timeline, optionally scoped to one entity.
    pub fn timeline(&self, entity: Option<&str>) -> Result<KgTimelineResult> {
        self.store.kg_timeline(entity)
    }

    /// Returns high-level KG statistics for audit/reporting.
    pub fn stats(&self) -> Result<KgStats> {
        self.store.kg_stats()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::storage::sqlite::SqliteStore;

    #[test]
    fn knowledge_graph_round_trip_and_stats_work() {
        let tmp = tempdir().unwrap();
        let sqlite = SqliteStore::open(&tmp.path().join("palace.sqlite3")).unwrap();
        sqlite.init_schema().unwrap();

        let kg = KnowledgeGraph::new(&sqlite);
        kg.add_triple(&KgTriple {
            subject: "Max".to_string(),
            predicate: "works_on".to_string(),
            object: "Mempalace".to_string(),
            valid_from: Some("2026-04-18".to_string()),
            valid_to: None,
        })
        .unwrap();

        let query = kg.query_entity("Max", None, "both").unwrap();
        assert_eq!(query.count, 1);
        assert_eq!(query.facts[0].predicate, "works_on");

        let stats = kg.stats().unwrap();
        assert_eq!(stats.triples, 1);
        assert_eq!(stats.current_facts, 1);

        let invalidated = kg
            .invalidate("Max", "works_on", "Mempalace", Some("2026-04-19"))
            .unwrap();
        assert_eq!(invalidated.updated, 1);

        let timeline = kg.timeline(Some("Max")).unwrap();
        assert_eq!(timeline.count, 1);
        assert_eq!(timeline.timeline[0].valid_to.as_deref(), Some("2026-04-19"));
    }
}
