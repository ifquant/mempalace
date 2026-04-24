use chrono::Utc;
use rusqlite::{OptionalExtension, params};

use crate::error::Result;
use crate::model::{
    DiaryEntry, DiaryReadResult, DiaryWriteResult, KgEntityWriteResult, KgFact, KgInvalidateResult,
    KgStats, KgTimelineResult, KgTriple, KgWriteResult,
};

use super::SqliteStore;

impl SqliteStore {
    pub fn add_kg_entity(&self, name: &str, entity_type: &str) -> Result<KgEntityWriteResult> {
        let entity_id = normalize_entity_id(name);
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO kg_entities(entity_id, name, entity_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(entity_id) DO UPDATE SET
                 name = excluded.name,
                 entity_type = excluded.entity_type,
                 updated_at = excluded.updated_at",
            params![entity_id, name, entity_type, now, now],
        )?;
        Ok(KgEntityWriteResult {
            success: true,
            entity_id,
            name: name.to_string(),
            entity_type: entity_type.to_string(),
        })
    }

    pub fn add_kg_triple(&self, triple: &KgTriple) -> Result<KgWriteResult> {
        self.add_kg_entity(&triple.subject, "unknown")?;
        self.add_kg_entity(&triple.object, "unknown")?;

        let existing_id = self
            .conn
            .query_row(
                "SELECT id FROM kg_triples
                 WHERE subject = ?1 AND predicate = ?2 AND object = ?3 AND valid_to IS NULL
                 LIMIT 1",
                params![triple.subject, triple.predicate, triple.object],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        if let Some(existing_id) = existing_id {
            return Ok(KgWriteResult {
                success: true,
                triple_id: kg_triple_id(
                    &triple.subject,
                    &triple.predicate,
                    &triple.object,
                    existing_id,
                ),
                fact: format!(
                    "{} → {} → {}",
                    triple.subject, triple.predicate, triple.object
                ),
            });
        }

        let created_at = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO kg_triples(subject, predicate, object, valid_from, valid_to, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                triple.subject,
                triple.predicate,
                triple.object,
                triple.valid_from,
                triple.valid_to,
                created_at,
            ],
        )?;
        let triple_row_id = self.conn.last_insert_rowid();
        Ok(KgWriteResult {
            success: true,
            triple_id: kg_triple_id(
                &triple.subject,
                &triple.predicate,
                &triple.object,
                triple_row_id,
            ),
            fact: format!(
                "{} → {} → {}",
                triple.subject, triple.predicate, triple.object
            ),
        })
    }

    pub fn invalidate_kg_triple(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
        ended: Option<&str>,
    ) -> Result<KgInvalidateResult> {
        let ended_value = ended
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
        let updated = self.conn.execute(
            "UPDATE kg_triples
             SET valid_to = ?1
             WHERE subject = ?2 AND predicate = ?3 AND object = ?4 AND valid_to IS NULL",
            params![ended_value, subject, predicate, object],
        )?;
        Ok(KgInvalidateResult {
            success: true,
            fact: format!("{subject} → {predicate} → {object}"),
            ended: ended_value,
            updated,
        })
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

    pub fn query_kg_entity(
        &self,
        entity: &str,
        as_of: Option<&str>,
        direction: &str,
    ) -> Result<Vec<KgFact>> {
        let mut results = Vec::new();

        if matches!(direction, "outgoing" | "both") {
            if let Some(as_of) = as_of {
                let mut stmt = self.conn.prepare(
                    "SELECT subject, predicate, object, valid_from, valid_to
                     FROM kg_triples
                     WHERE subject = ?1
                       AND (valid_from IS NULL OR valid_from <= ?2)
                       AND (valid_to IS NULL OR valid_to >= ?3)
                     ORDER BY id",
                )?;
                let rows = stmt.query_map([entity, as_of, as_of], |row| {
                    let valid_to: Option<String> = row.get(4)?;
                    Ok(KgFact {
                        direction: "outgoing".to_string(),
                        subject: row.get(0)?,
                        predicate: row.get(1)?,
                        object: row.get(2)?,
                        valid_from: row.get(3)?,
                        valid_to: valid_to.clone(),
                        current: valid_to.is_none(),
                    })
                })?;
                for row in rows {
                    results.push(row?);
                }
            } else {
                let mut stmt = self.conn.prepare(
                    "SELECT subject, predicate, object, valid_from, valid_to
                     FROM kg_triples WHERE subject = ?1 ORDER BY id",
                )?;
                let rows = stmt.query_map([entity], |row| {
                    let valid_to: Option<String> = row.get(4)?;
                    Ok(KgFact {
                        direction: "outgoing".to_string(),
                        subject: row.get(0)?,
                        predicate: row.get(1)?,
                        object: row.get(2)?,
                        valid_from: row.get(3)?,
                        valid_to: valid_to.clone(),
                        current: valid_to.is_none(),
                    })
                })?;
                for row in rows {
                    results.push(row?);
                }
            }
        }

        if matches!(direction, "incoming" | "both") {
            if let Some(as_of) = as_of {
                let mut stmt = self.conn.prepare(
                    "SELECT subject, predicate, object, valid_from, valid_to
                     FROM kg_triples
                     WHERE object = ?1
                       AND (valid_from IS NULL OR valid_from <= ?2)
                       AND (valid_to IS NULL OR valid_to >= ?3)
                     ORDER BY id",
                )?;
                let rows = stmt.query_map([entity, as_of, as_of], |row| {
                    let valid_to: Option<String> = row.get(4)?;
                    Ok(KgFact {
                        direction: "incoming".to_string(),
                        subject: row.get(0)?,
                        predicate: row.get(1)?,
                        object: row.get(2)?,
                        valid_from: row.get(3)?,
                        valid_to: valid_to.clone(),
                        current: valid_to.is_none(),
                    })
                })?;
                for row in rows {
                    results.push(row?);
                }
            } else {
                let mut stmt = self.conn.prepare(
                    "SELECT subject, predicate, object, valid_from, valid_to
                     FROM kg_triples WHERE object = ?1 ORDER BY id",
                )?;
                let rows = stmt.query_map([entity], |row| {
                    let valid_to: Option<String> = row.get(4)?;
                    Ok(KgFact {
                        direction: "incoming".to_string(),
                        subject: row.get(0)?,
                        predicate: row.get(1)?,
                        object: row.get(2)?,
                        valid_from: row.get(3)?,
                        valid_to: valid_to.clone(),
                        current: valid_to.is_none(),
                    })
                })?;
                for row in rows {
                    results.push(row?);
                }
            }
        }

        Ok(results)
    }

    pub fn kg_timeline(&self, entity: Option<&str>) -> Result<KgTimelineResult> {
        let mut query = String::from(
            "SELECT subject, predicate, object, valid_from, valid_to
             FROM kg_triples",
        );
        let rows = if let Some(entity) = entity {
            query.push_str(
                " WHERE subject = ?1 OR object = ?2
                  ORDER BY valid_from IS NULL, valid_from ASC, id ASC
                  LIMIT 100",
            );
            let mut stmt = self.conn.prepare(&query)?;
            stmt.query_map([entity, entity], |row| {
                let valid_to: Option<String> = row.get(4)?;
                Ok(KgFact {
                    direction: "both".to_string(),
                    subject: row.get(0)?,
                    predicate: row.get(1)?,
                    object: row.get(2)?,
                    valid_from: row.get(3)?,
                    valid_to: valid_to.clone(),
                    current: valid_to.is_none(),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            query.push_str(" ORDER BY valid_from IS NULL, valid_from ASC, id ASC LIMIT 100");
            let mut stmt = self.conn.prepare(&query)?;
            stmt.query_map([], |row| {
                let valid_to: Option<String> = row.get(4)?;
                Ok(KgFact {
                    direction: "both".to_string(),
                    subject: row.get(0)?,
                    predicate: row.get(1)?,
                    object: row.get(2)?,
                    valid_from: row.get(3)?,
                    valid_to: valid_to.clone(),
                    current: valid_to.is_none(),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?
        };

        Ok(KgTimelineResult {
            entity: entity.unwrap_or("all").to_string(),
            count: rows.len(),
            timeline: rows,
        })
    }

    pub fn kg_stats(&self) -> Result<KgStats> {
        let triples = self
            .conn
            .query_row("SELECT COUNT(*) FROM kg_triples", [], |row| {
                row.get::<_, i64>(0)
            })? as usize;
        let current_facts = self.conn.query_row(
            "SELECT COUNT(*) FROM kg_triples WHERE valid_to IS NULL",
            [],
            |row| row.get::<_, i64>(0),
        )? as usize;
        let expired_facts = triples.saturating_sub(current_facts);
        let entities = self
            .conn
            .query_row("SELECT COUNT(*) FROM kg_entities", [], |row| {
                row.get::<_, i64>(0)
            })? as usize;
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT predicate FROM kg_triples ORDER BY predicate")?;
        let relationship_types = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(KgStats {
            entities,
            triples,
            current_facts,
            expired_facts,
            relationship_types,
        })
    }

    pub fn add_diary_entry(
        &self,
        agent_name: &str,
        topic: &str,
        entry: &str,
    ) -> Result<DiaryWriteResult> {
        let wing = format!("wing_{}", normalize_agent_name(agent_name));
        let room = "diary";
        let timestamp = Utc::now().to_rfc3339();
        let date = timestamp.split('T').next().unwrap_or_default().to_string();
        let entry_id = format!(
            "diary_{wing}_{}_{}",
            date.replace('-', ""),
            blake3::hash(format!("{agent_name}|{topic}|{timestamp}|{entry}").as_bytes()).to_hex()
        );
        self.conn.execute(
            "INSERT INTO diary_entries (id, agent_name, wing, room, topic, entry, timestamp, date, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entry_id,
                agent_name,
                wing,
                room,
                topic,
                entry,
                timestamp,
                date,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(DiaryWriteResult {
            success: true,
            entry_id,
            agent: agent_name.to_string(),
            topic: topic.to_string(),
            timestamp,
        })
    }

    pub fn read_diary_entries(&self, agent_name: &str, last_n: usize) -> Result<DiaryReadResult> {
        let total = self.conn.query_row(
            "SELECT COUNT(*) FROM diary_entries WHERE agent_name = ?1",
            [agent_name],
            |row| row.get::<_, i64>(0),
        )? as usize;

        let mut stmt = self.conn.prepare(
            "SELECT date, timestamp, topic, entry
             FROM diary_entries
             WHERE agent_name = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;
        let entries = stmt
            .query_map(params![agent_name, last_n as i64], |row| {
                Ok(DiaryEntry {
                    date: row.get(0)?,
                    timestamp: row.get(1)?,
                    topic: row.get(2)?,
                    content: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(DiaryReadResult {
            agent: agent_name.to_string(),
            total,
            showing: entries.len(),
            message: if total == 0 {
                Some("No diary entries yet.".to_string())
            } else {
                None
            },
            entries,
        })
    }
}

fn normalize_agent_name(agent_name: &str) -> String {
    agent_name.trim().to_lowercase().replace([' ', '-'], "_")
}

fn kg_triple_id(subject: &str, predicate: &str, object: &str, row_id: i64) -> String {
    format!(
        "t_{}_{}_{}_{}",
        normalize_entity_fragment(subject),
        normalize_entity_fragment(predicate),
        normalize_entity_fragment(object),
        row_id
    )
}

fn normalize_entity_id(value: &str) -> String {
    let mut normalized = value
        .trim()
        .to_lowercase()
        .replace([' ', '-'], "_")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))
        .collect::<String>();
    if normalized.is_empty() {
        normalized = "item".to_string();
    }
    normalized
}

fn normalize_entity_fragment(value: &str) -> String {
    normalize_entity_id(value)
}
