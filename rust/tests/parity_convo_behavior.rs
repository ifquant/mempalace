use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::convo::extract_general_memories;
use mempalace_rs::model::MineRequest;
use mempalace_rs::service::App;
use mempalace_rs::storage::sqlite::SqliteStore;
use tempfile::tempdir;

#[tokio::test]
async fn parity_convo_mining_replaces_existing_source_chunks() {
    let tmp = tempdir().unwrap();
    let convo_dir = tmp.path().join("convos");
    std::fs::create_dir_all(&convo_dir).unwrap();
    let transcript = convo_dir.join("session.txt");
    std::fs::write(
        &transcript,
        "> first question\nFirst answer with technical code details.\n\n> second question\nSecond answer about architecture and deploy.\n",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    app.mine_project(
        &convo_dir,
        &MineRequest {
            wing: Some("chatlogs".to_string()),
            mode: "convos".to_string(),
            agent: "mempalace".to_string(),
            limit: 0,
            dry_run: false,
            respect_gitignore: true,
            include_ignored: vec![],
            extract: "exchange".to_string(),
        },
    )
    .await
    .unwrap();

    let sqlite = SqliteStore::open(&config.sqlite_path()).unwrap();
    assert_eq!(sqlite.total_drawers().unwrap(), 2);
    drop(sqlite);

    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(
        &transcript,
        "Human: what changed?\nAssistant: We switched to one room and one answer.\n",
    )
    .unwrap();

    app.mine_project(
        &convo_dir,
        &MineRequest {
            wing: Some("chatlogs".to_string()),
            mode: "convos".to_string(),
            agent: "mempalace".to_string(),
            limit: 0,
            dry_run: false,
            respect_gitignore: true,
            include_ignored: vec![],
            extract: "exchange".to_string(),
        },
    )
    .await
    .unwrap();

    let sqlite = SqliteStore::open(&config.sqlite_path()).unwrap();
    let drawers = sqlite.list_drawers(Some("chatlogs")).unwrap();
    assert_eq!(drawers.len(), 1);
    assert_eq!(
        drawers[0].source_path,
        transcript.canonicalize().unwrap().display().to_string()
    );
    assert_eq!(drawers[0].chunk_index, 0);
    drop(sqlite);

    let search = app
        .search("one room one answer", Some("chatlogs"), None, 10)
        .await
        .unwrap();
    assert_eq!(search.results.len(), 1);
    assert!(search.results[0].text.contains("one room and one answer"));
}

#[test]
fn parity_general_extractor_keeps_positive_resolved_text_out_of_problem() {
    let text = "The deployment problem was brutal, but we fixed it and now I feel proud and grateful that the rewrite is stable and beautiful.";
    let memories = extract_general_memories(text, 0.3);

    assert_eq!(memories.len(), 1);
    assert_ne!(memories[0].room, "problem");
    assert_eq!(memories[0].room, "emotional");
}

#[tokio::test]
async fn parity_wake_up_preserves_identity_and_kind() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let identity_path = config.identity_path();
    std::fs::create_dir_all(identity_path.parent().unwrap()).unwrap();
    std::fs::write(&identity_path, "I am Atlas.\nI protect continuity.").unwrap();

    let app = App::new(config).unwrap();
    app.init().await.unwrap();
    app.add_drawer(
        "project",
        "general",
        "We shipped the first stable rewrite milestone.",
        Some("notes.txt"),
        Some("mempalace"),
    )
    .await
    .unwrap();

    let wake = app.wake_up(Some("project")).await.unwrap();

    assert_eq!(wake.kind, "wake_up");
    assert_eq!(wake.wing.as_deref(), Some("project"));
    assert_eq!(wake.identity, "I am Atlas.\nI protect continuity.");
    assert!(wake.layer1.contains("stable rewrite"));
}
