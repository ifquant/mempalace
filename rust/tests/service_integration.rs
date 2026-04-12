use futures::TryStreamExt;
use std::sync::Arc;

use arrow_schema::{DataType, Field, Schema};
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::model::{KgTriple, MineRequest};
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
    app.mine_project(
        &project,
        &MineRequest {
            wing: Some("project".to_string()),
            mode: "projects".to_string(),
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
        .mine_project(
            &project,
            &MineRequest {
                wing: None,
                mode: "projects".to_string(),
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
    assert_eq!(summary.kind, "mine");
    assert_eq!(summary.mode, "projects");
    assert_eq!(summary.extract, "exchange");
    assert_eq!(summary.agent, "mempalace");
    assert_eq!(summary.wing, "alpha");
    assert_eq!(summary.configured_rooms, vec!["auth", "docs"]);
    assert_eq!(summary.project_path, project.display().to_string());
    assert_eq!(summary.version, env!("CARGO_PKG_VERSION"));
    assert!(!summary.dry_run);
    assert!(summary.respect_gitignore);
    assert!(summary.include_ignored.is_empty());
    assert_eq!(summary.files_planned, 2);
    assert_eq!(summary.files_seen, 2);
    assert_eq!(summary.files_mined, 2);
    assert_eq!(summary.room_counts["auth"], 1);
    assert_eq!(summary.room_counts["docs"], 1);
    assert_eq!(
        summary.next_hint,
        "mempalace search \"what you're looking for\""
    );

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
        .mine_project(
            &project,
            &MineRequest {
                wing: None,
                mode: "projects".to_string(),
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
    assert_eq!(skipped.files_seen, 0);

    let included = app
        .mine_project(
            &project,
            &MineRequest {
                wing: None,
                mode: "projects".to_string(),
                agent: "mempalace".to_string(),
                limit: 0,
                dry_run: false,
                respect_gitignore: true,
                include_ignored: vec![String::from("secret/plan.md")],
                extract: "exchange".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(included.files_seen, 1);
    assert_eq!(included.files_mined, 1);
    assert_eq!(included.room_counts["secrets"], 1);

    let taxonomy = app.taxonomy().await.unwrap();
    assert!(taxonomy.taxonomy["forced"].contains_key("secrets"));
}

#[tokio::test]
async fn mine_dry_run_reports_work_without_writing_drawers() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("src").join("notes.md"),
        "Graph search notes.\n\nDry run should not persist drawers.",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();
    app.init().await.unwrap();

    let summary = app
        .mine_project(
            &project,
            &MineRequest {
                wing: Some("project".to_string()),
                mode: "projects".to_string(),
                agent: "codex".to_string(),
                limit: 0,
                dry_run: true,
                respect_gitignore: true,
                include_ignored: vec![],
                extract: "exchange".to_string(),
            },
        )
        .await
        .unwrap();
    let status = app.status().await.unwrap();

    assert!(summary.dry_run);
    assert_eq!(summary.agent, "codex");
    assert_eq!(summary.mode, "projects");
    assert_eq!(summary.configured_rooms, vec!["general"]);
    assert_eq!(summary.files_planned, 1);
    assert_eq!(summary.files_seen, 1);
    assert_eq!(summary.files_mined, 1);
    assert!(summary.drawers_added > 0);
    assert_eq!(summary.room_counts["general"], 1);
    assert_eq!(status.total_drawers, 0);
    assert!(status.wings.is_empty());
}

#[tokio::test]
async fn mine_skips_unchanged_files_and_remines_when_mtime_changes() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("src")).unwrap();
    let source = project.join("src").join("cache.txt");
    std::fs::write(
        &source,
        "JWT authentication notes.\n\nModified-time parity matters here.",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();
    app.init().await.unwrap();

    let request = MineRequest {
        wing: Some("project".to_string()),
        mode: "projects".to_string(),
        agent: "mempalace".to_string(),
        limit: 0,
        dry_run: false,
        respect_gitignore: true,
        include_ignored: vec![],
        extract: "exchange".to_string(),
    };

    let first = app.mine_project(&project, &request).await.unwrap();
    let second = app.mine_project(&project, &request).await.unwrap();
    assert_eq!(first.files_mined, 1);
    assert_eq!(second.files_mined, 0);
    assert_eq!(second.files_skipped_unchanged, 1);

    let metadata = std::fs::metadata(&source).unwrap();
    let modified = metadata.modified().unwrap();
    let bumped = modified + std::time::Duration::from_secs(5);
    filetime::set_file_mtime(&source, filetime::FileTime::from_system_time(bumped)).unwrap();

    let third = app.mine_project(&project, &request).await.unwrap();
    let status = app.status().await.unwrap();
    assert_eq!(third.files_mined, 1);
    assert_eq!(third.files_skipped_unchanged, 0);
    assert!(status.total_drawers > 0);
}

#[tokio::test]
async fn mine_persists_python_style_drawer_metadata() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("src")).unwrap();
    let source = project.join("src").join("auth.txt");
    std::fs::write(
        &source,
        "JWT authentication tokens are stored locally.\n\nThe team switched to Clerk for auth.",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let sqlite_path = config.sqlite_path();
    let app = App::new(config).unwrap();
    app.init().await.unwrap();
    app.mine_project(
        &project,
        &MineRequest {
            wing: Some("project".to_string()),
            mode: "projects".to_string(),
            agent: "codex".to_string(),
            limit: 0,
            dry_run: false,
            respect_gitignore: true,
            include_ignored: vec![],
            extract: "exchange".to_string(),
        },
    )
    .await
    .unwrap();

    let conn = Connection::open(sqlite_path).unwrap();
    let (source_file, source_mtime, added_by, filed_at, created_at): (
        String,
        Option<f64>,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT source_file, source_mtime, added_by, filed_at, created_at FROM drawers LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();

    assert_eq!(source_file, "auth.txt");
    assert!(source_mtime.is_some());
    assert_eq!(added_by, "codex");
    assert!(!filed_at.is_empty());
    assert!(!created_at.is_empty());
}

#[tokio::test]
async fn mine_persists_python_style_metadata_into_vector_store() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("src").join("auth.txt"),
        "JWT authentication tokens are stored locally.\n\nThe team switched to Clerk for auth.",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let lance_path = config.lance_path();
    let app = App::new(config).unwrap();
    app.init().await.unwrap();
    app.mine_project(
        &project,
        &MineRequest {
            wing: Some("project".to_string()),
            mode: "projects".to_string(),
            agent: "codex".to_string(),
            limit: 0,
            dry_run: false,
            respect_gitignore: true,
            include_ignored: vec![],
            extract: "exchange".to_string(),
        },
    )
    .await
    .unwrap();

    let conn = connect(lance_path.to_string_lossy().as_ref())
        .execute()
        .await
        .unwrap();
    let table = conn.open_table("drawers").execute().await.unwrap();
    let schema = table.schema().await.unwrap();
    assert!(schema.field_with_name("source_file").is_ok());
    assert!(schema.field_with_name("source_mtime").is_ok());
    assert!(schema.field_with_name("added_by").is_ok());
    assert!(schema.field_with_name("filed_at").is_ok());

    let batches = table
        .query()
        .select(Select::columns(&[
            "source_file",
            "source_mtime",
            "added_by",
            "filed_at",
        ]))
        .execute()
        .await
        .unwrap()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    let batch = &batches[0];
    let source_file = batch["source_file"]
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .unwrap()
        .value(0)
        .to_string();
    let source_mtime = batch["source_mtime"]
        .as_any()
        .downcast_ref::<arrow_array::Float64Array>()
        .unwrap()
        .value(0);
    let added_by = batch["added_by"]
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .unwrap()
        .value(0)
        .to_string();
    let filed_at = batch["filed_at"]
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .unwrap()
        .value(0)
        .to_string();

    assert_eq!(source_file, "auth.txt");
    assert!(source_mtime > 0.0);
    assert_eq!(added_by, "codex");
    assert!(!filed_at.is_empty());

    let search = app
        .search("Clerk auth", Some("project"), None, 3)
        .await
        .unwrap();
    let first = &search.results[0];
    assert_eq!(first.source_file, "auth.txt");
    assert_eq!(first.added_by.as_deref(), Some("codex"));
    assert!(first.source_mtime.is_some());
    assert!(
        first
            .filed_at
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    );
}

#[tokio::test]
async fn init_upgrades_legacy_vector_table_with_python_style_metadata_columns() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    config.ensure_dirs().unwrap();

    let conn = connect(config.lance_path().to_string_lossy().as_ref())
        .execute()
        .await
        .unwrap();
    let legacy_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("wing", DataType::Utf8, false),
        Field::new("room", DataType::Utf8, false),
        Field::new("source_path", DataType::Utf8, false),
        Field::new("chunk_index", DataType::Int32, false),
        Field::new("text", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 64),
            true,
        ),
    ]));
    conn.create_empty_table("drawers", legacy_schema)
        .execute()
        .await
        .unwrap();

    let app = App::new(config).unwrap();
    app.init().await.unwrap();

    let conn = connect(app.config.lance_path().to_string_lossy().as_ref())
        .execute()
        .await
        .unwrap();
    let table = conn.open_table("drawers").execute().await.unwrap();
    let schema = table.schema().await.unwrap();
    assert!(schema.field_with_name("source_file").is_ok());
    assert!(schema.field_with_name("source_mtime").is_ok());
    assert!(schema.field_with_name("added_by").is_ok());
    assert!(schema.field_with_name("filed_at").is_ok());
}

#[tokio::test]
async fn mine_respects_nested_gitignore_and_negation_rules() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("subrepo").join("src")).unwrap();
    std::fs::create_dir_all(project.join("subrepo").join("tasks")).unwrap();
    std::fs::write(project.join(".gitignore"), "*.log\n").unwrap();
    std::fs::write(project.join("subrepo").join(".gitignore"), "tasks/\n").unwrap();
    std::fs::write(
        project.join("subrepo").join("src").join("main.py"),
        "print('main')\nprint('main')\nprint('main')\nprint('main')\nprint('main')\n",
    )
    .unwrap();
    std::fs::write(
        project.join("subrepo").join("tasks").join("task.py"),
        "print('task')\nprint('task')\nprint('task')\nprint('task')\nprint('task')\n",
    )
    .unwrap();
    std::fs::write(
        project.join("subrepo").join("debug.log"),
        "debug\n".repeat(20),
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();
    app.init().await.unwrap();

    let summary = app
        .mine_project(
            &project,
            &MineRequest {
                wing: Some("nested".to_string()),
                mode: "projects".to_string(),
                agent: "mempalace".to_string(),
                limit: 0,
                dry_run: true,
                respect_gitignore: true,
                include_ignored: vec![],
                extract: "exchange".to_string(),
            },
        )
        .await
        .unwrap();

    assert_eq!(summary.files_seen, 1);
    assert_eq!(summary.files_mined, 1);
}

#[tokio::test]
async fn mine_handles_gitignore_negation_only_when_parent_dir_remains_visible() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("generated")).unwrap();
    std::fs::write(
        project.join(".gitignore"),
        "generated/*\n!generated/keep.py\n",
    )
    .unwrap();
    std::fs::write(
        project.join("generated").join("drop.py"),
        "print('drop')\n".repeat(20),
    )
    .unwrap();
    std::fs::write(
        project.join("generated").join("keep.py"),
        "print('keep')\n".repeat(20),
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();
    app.init().await.unwrap();

    let summary = app
        .mine_project(
            &project,
            &MineRequest {
                wing: Some("negation".to_string()),
                mode: "projects".to_string(),
                agent: "mempalace".to_string(),
                limit: 0,
                dry_run: true,
                respect_gitignore: true,
                include_ignored: vec![],
                extract: "exchange".to_string(),
            },
        )
        .await
        .unwrap();

    assert_eq!(summary.files_seen, 1);
    assert_eq!(summary.files_mined, 1);
}

#[tokio::test]
async fn mine_does_not_reinclude_file_from_ignored_directory_without_override() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("generated")).unwrap();
    std::fs::write(
        project.join(".gitignore"),
        "generated/\n!generated/keep.py\n",
    )
    .unwrap();
    std::fs::write(
        project.join("generated").join("drop.py"),
        "print('drop')\n".repeat(20),
    )
    .unwrap();
    std::fs::write(
        project.join("generated").join("keep.py"),
        "print('keep')\n".repeat(20),
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();
    app.init().await.unwrap();

    let summary = app
        .mine_project(
            &project,
            &MineRequest {
                wing: Some("ignored-dir".to_string()),
                mode: "projects".to_string(),
                agent: "mempalace".to_string(),
                limit: 0,
                dry_run: true,
                respect_gitignore: true,
                include_ignored: vec![],
                extract: "exchange".to_string(),
            },
        )
        .await
        .unwrap();

    assert_eq!(summary.files_seen, 0);
    assert_eq!(summary.files_mined, 0);
}

#[tokio::test]
async fn mine_include_override_beats_skip_dirs_without_gitignore() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join(".pytest_cache")).unwrap();
    std::fs::write(
        project.join(".pytest_cache").join("cache.py"),
        "print('cache')\n".repeat(20),
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config).unwrap();
    app.init().await.unwrap();

    let summary = app
        .mine_project(
            &project,
            &MineRequest {
                wing: Some("skipdir".to_string()),
                mode: "projects".to_string(),
                agent: "mempalace".to_string(),
                limit: 0,
                dry_run: true,
                respect_gitignore: false,
                include_ignored: vec![".pytest_cache".to_string()],
                extract: "exchange".to_string(),
            },
        )
        .await
        .unwrap();

    assert_eq!(summary.files_seen, 1);
    assert_eq!(summary.files_mined, 1);
    assert_eq!(summary.room_counts["general"], 1);
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
    let drawer_columns: i64 = Connection::open(config.sqlite_path())
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('drawers') WHERE name IN ('source_file', 'source_mtime', 'added_by', 'filed_at')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(drawer_columns, 4);
}
