use std::fs;

use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::layers::LayerStack;
use mempalace_rs::model::DrawerInput;
use mempalace_rs::service::App;
use mempalace_rs::storage::sqlite::SqliteStore;
use tempfile::tempdir;

#[tokio::test]
async fn layer0_trims_identity_and_matches_python_token_estimate() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let padded_identity = format!("  {}  \n\n", "A".repeat(400));
    fs::write(config.identity_path(), &padded_identity).unwrap();

    let stack = LayerStack::with_app(app);
    let layer0 = stack.layer0().await.unwrap();
    let wake = stack.wake_up(None).await.unwrap();

    assert_eq!(layer0.identity, "A".repeat(400));
    assert_eq!(layer0.token_estimate, 100);
    assert_eq!(wake.identity, "A".repeat(400));
    assert!(wake.layer1.contains("No memories yet"));
}

#[tokio::test]
async fn dedup_dry_run_marks_short_docs_without_deleting_anything() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let sqlite = SqliteStore::open(&config.sqlite_path()).unwrap();
    let keeper = DrawerInput {
        id: "keep_long".to_string(),
        wing: "project".to_string(),
        room: "general".to_string(),
        source_file: "dedup-short.txt".to_string(),
        source_path: "dedup-short.txt".to_string(),
        source_hash: "keep".to_string(),
        source_mtime: None,
        chunk_index: 0,
        added_by: "codex".to_string(),
        filed_at: "2026-04-18T00:00:00Z".to_string(),
        ingest_mode: "projects".to_string(),
        extract_mode: "exchange".to_string(),
        importance: None,
        text: "long enough document to keep in the palace".to_string(),
    };
    let short = DrawerInput {
        id: "delete_short".to_string(),
        chunk_index: 1,
        source_hash: "short".to_string(),
        text: "tiny".to_string(),
        ..keeper.clone()
    };
    sqlite.insert_drawer(&keeper).unwrap();
    sqlite.insert_drawer(&short).unwrap();

    let vector = mempalace_rs::storage::vector::VectorStore::connect(&config.lance_path())
        .await
        .unwrap();
    vector
        .add_drawers(
            &[keeper.clone(), short.clone()],
            &[vec![1.0; 64], vec![0.0; 64]],
        )
        .await
        .unwrap();

    let preview = app
        .dedup(0.01, true, None, Some("dedup-short.txt"), 2, false)
        .await
        .unwrap();

    assert!(preview.dry_run);
    assert_eq!(preview.sources_checked, 1);
    assert_eq!(preview.kept, 1);
    assert_eq!(preview.deleted, 1);
    assert_eq!(preview.groups.len(), 1);
    assert_eq!(preview.groups[0].source_file, "dedup-short.txt");
    assert_eq!(preview.groups[0].kept, 1);
    assert_eq!(preview.groups[0].deleted, 1);

    let sqlite = SqliteStore::open(&config.sqlite_path()).unwrap();
    assert_eq!(sqlite.total_drawers().unwrap(), 2);
}

#[tokio::test]
async fn repair_prune_preview_reports_queue_without_mutating_palace() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    app.add_drawer(
        "project",
        "general",
        "This stable drawer should survive prune preview.",
        Some("repair-preview.txt"),
        Some("codex"),
    )
    .await
    .unwrap();

    fs::write(
        config.palace_path.join("corrupt_ids.txt"),
        "orphan_a\n\n orphan_b \n",
    )
    .unwrap();

    let preview = app.repair_prune(false).await.unwrap();

    assert_eq!(preview.queued, 2);
    assert!(!preview.confirm);
    assert_eq!(preview.deleted_from_vector, 0);
    assert_eq!(preview.deleted_from_sqlite, 0);
    assert_eq!(preview.failed, 0);

    let sqlite = SqliteStore::open(&config.sqlite_path()).unwrap();
    assert_eq!(sqlite.total_drawers().unwrap(), 1);
    assert_eq!(
        fs::read_to_string(config.palace_path.join("corrupt_ids.txt")).unwrap(),
        "orphan_a\n\n orphan_b \n"
    );
}

#[tokio::test]
async fn repair_prune_live_reports_failures_for_ids_missing_from_both_stores() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    fs::write(
        config.palace_path.join("corrupt_ids.txt"),
        "missing-one\nmissing-two\n",
    )
    .unwrap();

    let summary = app.repair_prune(true).await.unwrap();

    assert_eq!(summary.queued, 2);
    assert!(summary.confirm);
    assert_eq!(summary.failed, 2);
}

#[test]
fn layer1_stops_at_python_style_global_char_cap() {
    use mempalace_rs::layers::render_layer1;
    use mempalace_rs::storage::sqlite::DrawerRecord;

    let drawers = (0..30)
        .map(|i| {
            let source_file = format!("f{i}-{}.txt", "x".repeat(120));
            DrawerRecord {
                id: format!("d{i}"),
                wing: "project".to_string(),
                room: "general".to_string(),
                source_file: source_file.clone(),
                source_path: format!("/tmp/{source_file}"),
                source_hash: format!("h{i}"),
                source_mtime: None,
                chunk_index: i,
                added_by: "codex".to_string(),
                filed_at: "2026-04-25T00:00:00Z".to_string(),
                ingest_mode: "projects".to_string(),
                extract_mode: "exchange".to_string(),
                importance: Some(5.0),
                text: "A".repeat(260),
            }
        })
        .collect::<Vec<_>>();

    let layer1 = render_layer1(&drawers, Some("project"));

    assert!(layer1.contains("## L1"));
    assert!(layer1.contains("more in L3 search"));
    assert!(layer1.chars().count() <= 3600);
}

#[test]
fn layer1_prefers_importance_then_weight_defaulting_to_three() {
    use mempalace_rs::layers::render_layer1;
    use mempalace_rs::storage::sqlite::DrawerRecord;

    let drawers = vec![
        DrawerRecord {
            id: "emotional".to_string(),
            wing: "project".to_string(),
            room: "general".to_string(),
            source_file: "emotional.txt".to_string(),
            source_path: "/tmp/emotional.txt".to_string(),
            source_hash: "h1".to_string(),
            source_mtime: None,
            chunk_index: 0,
            added_by: "codex".to_string(),
            filed_at: "2026-04-25T00:00:00Z".to_string(),
            ingest_mode: "projects".to_string(),
            extract_mode: "exchange".to_string(),
            importance: Some(5.0),
            text: "emotional drawer".to_string(),
        },
        DrawerRecord {
            id: "weight".to_string(),
            wing: "project".to_string(),
            room: "general".to_string(),
            source_file: "weight.txt".to_string(),
            source_path: "/tmp/weight.txt".to_string(),
            source_hash: "h2".to_string(),
            source_mtime: None,
            chunk_index: 1,
            added_by: "codex".to_string(),
            filed_at: "2026-04-25T00:00:00Z".to_string(),
            ingest_mode: "projects".to_string(),
            extract_mode: "exchange".to_string(),
            importance: Some(1.0),
            text: "weight drawer".to_string(),
        },
        DrawerRecord {
            id: "default".to_string(),
            wing: "project".to_string(),
            room: "general".to_string(),
            source_file: "default.txt".to_string(),
            source_path: "/tmp/default.txt".to_string(),
            source_hash: "h3".to_string(),
            source_mtime: None,
            chunk_index: 2,
            added_by: "codex".to_string(),
            filed_at: "2026-04-25T00:00:00Z".to_string(),
            ingest_mode: "projects".to_string(),
            extract_mode: "exchange".to_string(),
            importance: None,
            text: "default drawer".to_string(),
        },
    ];

    let layer1 = render_layer1(&drawers, Some("project"));

    let emotional_pos = layer1.find("emotional drawer").unwrap();
    let default_pos = layer1.find("default drawer").unwrap();
    let weight_pos = layer1.find("weight drawer").unwrap();

    assert!(emotional_pos < default_pos);
    assert!(default_pos < weight_pos);
}
