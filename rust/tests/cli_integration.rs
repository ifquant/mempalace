use std::fs;

use assert_cmd::Command;
use predicates::str::contains;
use rusqlite::Connection;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn cli_init_status_mine_search_round_trip() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("src").join("auth.txt"),
        "JWT authentication tokens are stored locally.\n\nThe team switched to Clerk for auth.",
    )
    .unwrap();

    let palace = tmp.path().join("palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "init",
            project.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("palace.sqlite3"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("\"files_mined\": 1"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "status"])
        .assert()
        .success()
        .stdout(contains("\"total_drawers\":"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "doctor"])
        .assert()
        .success()
        .stdout(contains("\"provider\": \"hash\""));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "prepare-embedding",
            "--attempts",
            "1",
        ])
        .assert()
        .success()
        .stdout(contains("\"success\": true"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "search",
            "Clerk auth",
            "--results",
            "3",
        ])
        .assert()
        .success()
        .stdout(contains("Clerk"));
}

#[test]
fn cli_root_help_mentions_core_commands_and_examples() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--help"])
        .assert()
        .success()
        .stdout(contains(
            "MemPalace — Give your AI a memory. No API key required.",
        ))
        .stdout(contains("mempalace-rs mine ~/projects/my_app"))
        .stdout(contains("migrate"))
        .stdout(contains("repair"));
}

#[test]
fn cli_search_help_mentions_filters_and_results() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(contains("Find anything, exact words"))
        .stdout(contains("Limit to one project/wing"))
        .stdout(contains("Number of results"));
}

#[test]
#[ignore = "requires fastembed runtime and model warm-up"]
fn cli_fastembed_prepare_mine_search_smoke() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("guides")).unwrap();
    fs::write(
        project.join("guides").join("search.txt"),
        "MemPalace uses LanceDB for local vector search. The Rust rewrite uses fastembed for semantic retrieval.",
    )
    .unwrap();

    let palace = tmp.path().join("palace");
    let hf_endpoint = std::env::var("MEMPALACE_RS_TEST_HF_ENDPOINT").ok();

    let prepare = run_cli_json(
        &palace,
        "prepare-embedding",
        &["--attempts", "1", "--wait-ms", "0"],
        hf_endpoint.as_deref(),
    );
    assert_eq!(prepare["provider"], "fastembed");
    assert_eq!(prepare["success"], true);
    assert_eq!(prepare["doctor"]["warmup_ok"], true);

    let mine = run_cli_json(
        &palace,
        "mine",
        &[project.to_str().unwrap()],
        hf_endpoint.as_deref(),
    );
    assert_eq!(mine["files_mined"], 1);
    assert!(mine["drawers_added"].as_u64().unwrap_or(0) > 0);

    let search = run_cli_json(
        &palace,
        "search",
        &["semantic retrieval", "--results", "3"],
        hf_endpoint.as_deref(),
    );
    assert_eq!(search["query"], "semantic retrieval");
    assert_eq!(search["filters"]["wing"], Value::Null);
    assert_eq!(search["filters"]["room"], Value::Null);
    let results = search["results"].as_array().unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|hit| {
        hit["text"]
            .as_str()
            .unwrap_or_default()
            .contains("fastembed")
    }));
}

#[test]
fn cli_migrate_upgrades_legacy_sqlite_schema() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace");
    fs::create_dir_all(&palace).unwrap();
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

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "migrate"])
        .assert()
        .success()
        .stdout(contains("\"schema_version_before\": 1"))
        .stdout(contains("\"schema_version_after\": 2"))
        .stdout(contains("\"changed\": true"));
}

#[test]
fn cli_repair_reports_missing_palace_non_destructively() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("missing-palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "repair"])
        .assert()
        .success()
        .stdout(contains("\"ok\": false"))
        .stdout(contains("SQLite palace file is missing"))
        .stdout(contains("LanceDB directory is missing"));
}

#[test]
fn cli_repair_reports_healthy_hash_palace() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("src").join("auth.txt"),
        "JWT authentication tokens are stored locally.\n\nThe team switched to Clerk for auth.",
    )
    .unwrap();
    let palace = tmp.path().join("palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "init",
            project.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "repair"])
        .assert()
        .success()
        .stdout(contains("\"ok\": true"))
        .stdout(contains("\"vector_accessible\": true"))
        .stdout(contains("\"embedding_provider\": \"hash\""))
        .stdout(contains("\"schema_version\": 2"));
}

#[test]
fn cli_search_json_matches_python_style_shape() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("src").join("auth.txt"),
        "JWT authentication tokens are stored locally.\n\nThe team switched to Clerk for auth.",
    )
    .unwrap();
    let palace = tmp.path().join("palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "init",
            project.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
            "--wing",
            "my_app",
        ])
        .assert()
        .success();

    let output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "search",
            "Clerk auth",
            "--wing",
            "my_app",
            "--results",
            "3",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let search: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(search["query"], "Clerk auth");
    assert_eq!(search["filters"]["wing"], "my_app");
    assert_eq!(search["filters"]["room"], Value::Null);
    let first = &search["results"].as_array().unwrap()[0];
    assert!(first["source_file"].as_str().unwrap().ends_with("auth.txt"));
    assert!(first.get("similarity").is_some());
}

fn run_cli_json(
    palace: &std::path::Path,
    command: &str,
    args: &[&str],
    hf_endpoint: Option<&str>,
) -> Value {
    let mut cmd = Command::cargo_bin("mempalace-rs").unwrap();
    cmd.env("MEMPALACE_RS_EMBED_PROVIDER", "fastembed")
        .env("MEMPALACE_RS_EMBED_MODEL", "MultilingualE5Small")
        .env("MEMPALACE_RS_EMBED_SHOW_DOWNLOAD_PROGRESS", "false")
        .arg("--palace")
        .arg(palace);

    if let Some(endpoint) = hf_endpoint {
        cmd.arg("--hf-endpoint").arg(endpoint);
    }

    cmd.arg(command).args(args);

    let output = cmd.assert().success().get_output().stdout.clone();
    serde_json::from_slice(&output).unwrap()
}
