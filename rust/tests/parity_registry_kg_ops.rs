use std::collections::BTreeMap;

use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::registry::{EntityRegistry, SeedPerson};
use mempalace_rs::service::App;
use tempfile::tempdir;

fn hash_app(root: &std::path::Path) -> App {
    let mut config = AppConfig::resolve(Some(root.join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    App::new(config).unwrap()
}

#[tokio::test]
async fn parity_kg_add_auto_creates_entities_and_updates_stats() {
    let tmp = tempdir().unwrap();
    let app = hash_app(tmp.path());

    let write = app.kg_add("Alice", "knows", "Bob", None).await.unwrap();
    let stats = app.kg_stats().await.unwrap();

    assert!(write.triple_id.starts_with("t_alice_knows_bob_"));
    assert_eq!(stats.entities, 2);
    assert_eq!(stats.triples, 1);
    assert_eq!(stats.current_facts, 1);
    assert_eq!(stats.expired_facts, 0);
    assert_eq!(stats.relationship_types, vec!["knows".to_string()]);
}

#[tokio::test]
async fn parity_registry_lookup_uses_context_to_disambiguate_name() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    let registry_path = project.join("entity_registry.json");
    let mut registry = EntityRegistry::empty("personal");
    registry.seed(
        "personal",
        &[SeedPerson {
            name: "Ever".to_string(),
            relationship: "friend".to_string(),
            context: "personal".to_string(),
        }],
        &[],
        &BTreeMap::new(),
    );
    registry.save(&registry_path).unwrap();

    let app = hash_app(tmp.path());
    let lookup = app
        .registry_lookup(&project, "Ever", "Ever said the project was ready.")
        .unwrap();

    assert_eq!(lookup.r#type, "person");
    assert_eq!(lookup.name, "Ever");
    assert_eq!(lookup.disambiguated_by.as_deref(), Some("context_patterns"));
    assert!(!lookup.needs_disambiguation);
}

#[tokio::test]
async fn parity_diary_read_for_new_agent_returns_python_style_empty_message() {
    let tmp = tempdir().unwrap();
    let app = hash_app(tmp.path());
    app.init().await.unwrap();

    let result = app.diary_read("Codex", 10).await.unwrap();

    assert_eq!(result.agent, "Codex");
    assert_eq!(result.entries, vec![]);
    assert_eq!(result.total, 0);
    assert_eq!(result.showing, 0);
    assert_eq!(result.message.as_deref(), Some("No diary entries yet."));
}
