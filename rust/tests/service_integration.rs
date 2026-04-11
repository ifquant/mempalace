use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::model::KgTriple;
use mempalace_rs::service::App;
use mempalace_rs::storage::sqlite::{CURRENT_SCHEMA_VERSION, SqliteStore};
use rusqlite::Connection;
use tempfile::tempdir;

#[tokio::test]
async fn init_is_idempotent_and_status_starts_empty() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();

    let first = app.init().await.unwrap();
    let second = app.init().await.unwrap();
    let status = app.status().await.unwrap();
    let doctor = app.doctor(false).await.unwrap();
    let prepare = app.prepare_embedding(1, 0).await.unwrap();

    assert_eq!(first.palace_path, second.palace_path);
    assert_eq!(first.kind, "init");
    assert_eq!(first.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(status.total_drawers, 0);
    assert!(status.wings.is_empty());
    assert!(status.rooms.is_empty());
    assert_eq!(status.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(doctor.kind, "doctor");
    assert_eq!(doctor.version, env!("CARGO_PKG_VERSION"));
    assert!(doctor.sqlite_path.ends_with("palace.sqlite3"));
    assert!(doctor.lance_path.ends_with("lance"));
    assert_eq!(prepare.kind, "prepare_embedding");
    assert_eq!(prepare.version, env!("CARGO_PKG_VERSION"));
    assert!(prepare.sqlite_path.ends_with("palace.sqlite3"));
    assert!(prepare.lance_path.ends_with("lance"));
}

#[tokio::test]
async fn kg_round_trip_and_taxonomy_work() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("src").join("graph.txt"),
        "Graph search notes.\n\nVector retrieval and taxonomy.",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();
    app.init().await.unwrap();
    app.mine_project(&project, Some("project"), 0, true, &[])
        .await
        .unwrap();

    let taxonomy = app.taxonomy().await.unwrap();
    assert_eq!(taxonomy.taxonomy["project"]["general"], 1);

    let triple = KgTriple {
        subject: "GraphQL".to_string(),
        predicate: "depends_on".to_string(),
        object: "Postgres".to_string(),
        valid_from: Some("2026-04-11T00:00:00Z".to_string()),
        valid_to: None,
    };
    app.add_kg_triple(&triple).await.unwrap();

    let triples = app.query_kg("GraphQL").await.unwrap();
    assert_eq!(triples.len(), 1);
    assert_eq!(triples[0].predicate, "depends_on");
    assert_eq!(triples[0].object, "Postgres");
}

#[tokio::test]
async fn mine_respects_project_config_room_detection_and_scan_rules() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::create_dir_all(project.join("docs")).unwrap();
    std::fs::create_dir_all(project.join("node_modules")).unwrap();
    std::fs::write(
        project.join("mempalace.yaml"),
        r#"
wing: alpha
rooms:
  - name: auth
    keywords: [jwt, clerk, token]
  - name: docs
    keywords: [guide, architecture]
"#,
    )
    .unwrap();
    std::fs::write(
        project.join("src").join("security.txt"),
        "JWT token rotation and Clerk auth flow are documented here.\n\nUse secure auth tokens everywhere.",
    )
    .unwrap();
    std::fs::write(
        project.join("docs").join("guide.md"),
        "Architecture guide for the Rust rewrite.\n\nThis guide explains room taxonomy and project docs.",
    )
    .unwrap();
    std::fs::write(
        project.join("notes.bin"),
        "this should not be scanned because the extension is not readable",
    )
    .unwrap();
    std::fs::write(
        project.join("node_modules").join("ignore.txt"),
        "this should be skipped",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();
    app.init().await.unwrap();

    let summary = app
        .mine_project(&project, None, 0, true, &[])
        .await
        .unwrap();
    assert_eq!(summary.kind, "mine");
    assert_eq!(summary.wing, "alpha");
    assert_eq!(summary.project_path, project.display().to_string());
    assert_eq!(summary.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(summary.files_seen, 2);
    assert_eq!(summary.files_mined, 2);

    let taxonomy = app.taxonomy().await.unwrap();
    assert!(taxonomy.taxonomy["alpha"].contains_key("auth"));
    assert!(taxonomy.taxonomy["alpha"].contains_key("docs"));
    assert!(!taxonomy.taxonomy["alpha"].contains_key("src"));
}

#[tokio::test]
async fn mine_can_force_include_gitignored_paths() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("secret")).unwrap();
    std::fs::write(project.join(".gitignore"), "secret/\n").unwrap();
    std::fs::write(
        project.join("mempalace.yaml"),
        r#"
wing: forced
rooms:
  - name: secrets
    keywords: [secret]
"#,
    )
    .unwrap();
    std::fs::write(
        project.join("secret").join("plan.md"),
        "Secret rollout plan.\n\nThis file should only be mined when force included.",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();
    app.init().await.unwrap();

    let skipped = app
        .mine_project(&project, None, 0, true, &[])
        .await
        .unwrap();
    assert_eq!(skipped.files_seen, 0);

    let included = app
        .mine_project(&project, None, 0, true, &[String::from("secret/plan.md")])
        .await
        .unwrap();
    assert_eq!(included.files_seen, 1);
    assert_eq!(included.files_mined, 1);

    let taxonomy = app.taxonomy().await.unwrap();
    assert!(taxonomy.taxonomy["forced"].contains_key("secrets"));
}

#[tokio::test]
async fn init_migrates_v1_sqlite_schema_to_current() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace");
    std::fs::create_dir_all(&palace).unwrap();
    let sqlite_path = palace.join("palace.sqlite3");

    let conn = Connection::open(&sqlite_path).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        INSERT INTO meta(key, value) VALUES('schema_version', '1');

        CREATE TABLE drawers (
            id TEXT PRIMARY KEY,
            wing TEXT NOT NULL,
            room TEXT NOT NULL,
            source_path TEXT NOT NULL,
            source_hash TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            text TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE ingested_files (
            source_path TEXT PRIMARY KEY,
            content_hash TEXT NOT NULL,
            wing TEXT NOT NULL,
            room TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE kg_triples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            subject TEXT NOT NULL,
            predicate TEXT NOT NULL,
            object TEXT NOT NULL,
            valid_from TEXT,
            valid_to TEXT,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .unwrap();
    drop(conn);

    let mut config = AppConfig::resolve(Some(&palace)).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let status = app.status().await.unwrap();
    assert_eq!(status.schema_version, CURRENT_SCHEMA_VERSION);

    let sqlite = SqliteStore::open(&config.sqlite_path()).unwrap();
    assert_eq!(
        sqlite.schema_version().unwrap(),
        Some(CURRENT_SCHEMA_VERSION)
    );
}
