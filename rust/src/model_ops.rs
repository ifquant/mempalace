use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KgTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KgFact {
    pub direction: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub current: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KgQueryResult {
    pub entity: String,
    pub as_of: Option<String>,
    pub facts: Vec<KgFact>,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KgTimelineResult {
    pub entity: String,
    pub timeline: Vec<KgFact>,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KgStats {
    pub entities: usize,
    pub triples: usize,
    pub current_facts: usize,
    pub expired_facts: usize,
    pub relationship_types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KgWriteResult {
    pub success: bool,
    pub triple_id: String,
    pub fact: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KgEntityWriteResult {
    pub success: bool,
    pub entity_id: String,
    pub name: String,
    pub entity_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KgInvalidateResult {
    pub success: bool,
    pub fact: String,
    pub ended: String,
    pub updated: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DiaryWriteResult {
    pub success: bool,
    pub entry_id: String,
    pub agent: String,
    pub topic: String,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DiaryEntry {
    pub date: String,
    pub timestamp: String,
    pub topic: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DiaryReadResult {
    pub agent: String,
    pub entries: Vec<DiaryEntry>,
    pub total: usize,
    pub showing: usize,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DrawerWriteResult {
    pub success: bool,
    pub drawer_id: String,
    pub wing: String,
    pub room: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DrawerDeleteResult {
    pub success: bool,
    pub drawer_id: String,
}
