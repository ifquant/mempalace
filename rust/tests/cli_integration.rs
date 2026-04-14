use std::fs;

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
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
        .stdout(contains("\"kind\": \"init\""))
        .stdout(contains("\"version\":"))
        .stdout(contains("\"schema_version\": 4"))
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
        .stdout(contains("\"kind\": \"mine\""))
        .stdout(contains("\"mode\": \"projects\""))
        .stdout(contains("\"extract\": \"exchange\""))
        .stdout(contains("\"agent\": \"mempalace\""))
        .stdout(contains("\"configured_rooms\":"))
        .stdout(contains("\"dry_run\": false"))
        .stdout(contains("\"project_path\":"))
        .stdout(contains("\"palace_path\":"))
        .stdout(contains("\"filters\":"))
        .stdout(contains("\"files_planned\": 1"))
        .stdout(contains("\"room_counts\":"))
        .stdout(contains("\"next_hint\":"))
        .stdout(contains("\"files_mined\": 1"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "status"])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"status\""))
        .stdout(contains("\"sqlite_path\":"))
        .stdout(contains("\"lance_path\":"))
        .stdout(contains("\"total_drawers\":"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "doctor"])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"doctor\""))
        .stdout(contains("\"version\":"))
        .stdout(contains("\"sqlite_path\":"))
        .stdout(contains("\"lance_path\":"))
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
        .stdout(contains("\"kind\": \"prepare_embedding\""))
        .stdout(contains("\"version\":"))
        .stdout(contains("\"sqlite_path\":"))
        .stdout(contains("\"lance_path\":"))
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
fn cli_init_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(contains("Set up a palace directory for a project"))
        .stdout(contains("human-readable init summary"));
}

#[test]
fn cli_doctor_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(contains("Inspect embedding runtime health and cache state"))
        .stdout(contains("human-readable doctor output"));
}

#[test]
fn cli_prepare_embedding_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["prepare-embedding", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Prepare the local embedding runtime and model cache",
        ))
        .stdout(contains("human-readable prepare summary"));
}

#[test]
fn cli_mine_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["mine", "--help"])
        .assert()
        .success()
        .stdout(contains("human-readable mine summary"));
}

#[test]
fn cli_status_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["status", "--help"])
        .assert()
        .success()
        .stdout(contains("Show what has been filed in the palace"))
        .stdout(contains("human-readable palace status"));
}

#[test]
fn cli_repair_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["repair", "--help"])
        .assert()
        .success()
        .stdout(contains("Run non-destructive palace diagnostics"))
        .stdout(contains("human-readable repair diagnostics"));
}

#[test]
fn cli_migrate_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["migrate", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Upgrade palace SQLite metadata to the current schema version",
        ))
        .stdout(contains("human-readable migration summary"));
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
        .stdout(contains("Number of results"))
        .stdout(contains("human-readable search output"));
}

#[test]
fn cli_init_human_prints_python_style_summary() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let palace = tmp.path().join("palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "init",
            project.to_str().unwrap(),
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Init"))
        .stdout(contains("Palace:"))
        .stdout(contains("SQLite:"))
        .stdout(contains("LanceDB:"))
        .stdout(contains("Schema:  4"))
        .stdout(contains("Palace initialized."));
}

#[test]
fn cli_doctor_human_prints_embedding_diagnostics() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
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
        .args(["--palace", palace.to_str().unwrap(), "doctor", "--human"])
        .assert()
        .success()
        .stdout(contains("MemPalace Doctor"))
        .stdout(contains("Palace:"))
        .stdout(contains("SQLite:"))
        .stdout(contains("LanceDB:"))
        .stdout(contains("Provider:   hash"))
        .stdout(contains("Model:      hash-v1"))
        .stdout(contains("Dimension:  64"));
}

#[test]
fn cli_prepare_embedding_human_prints_embedding_preparation_summary() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
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
            "prepare-embedding",
            "--attempts",
            "1",
            "--wait-ms",
            "0",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Prepare Embedding"))
        .stdout(contains("Palace:"))
        .stdout(contains("Provider:  hash"))
        .stdout(contains("Model:     hash-v1"))
        .stdout(contains("Attempts:  1"))
        .stdout(contains("Result:    ok"))
        .stdout(contains("Warmup:    ok"));
}

#[test]
fn cli_mine_help_mentions_mode_agent_and_extract() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["mine", "--help"])
        .assert()
        .success()
        .stdout(contains("Ingest mode: 'projects' for code/docs"))
        .stdout(contains("Your name"))
        .stdout(contains("Extraction strategy for convos mode"))
        .stdout(contains("per-file mining progress"))
        .stdout(contains("human-readable mine summary"));
}

#[test]
fn cli_status_reports_no_palace_with_python_style_hint() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("missing-palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "status"])
        .assert()
        .success()
        .stdout(contains("\"error\": \"No palace found\""))
        .stdout(contains(
            "Run: mempalace init <dir> && mempalace mine <dir>",
        ))
        .stdout(contains("\"palace_path\":"));
}

#[test]
fn cli_status_human_reports_no_palace_with_python_style_text() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("missing-palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "status", "--human"])
        .assert()
        .success()
        .stdout(contains("No palace found at"))
        .stdout(contains(
            "Run: mempalace init <dir> then mempalace mine <dir>",
        ));
}

#[test]
fn cli_search_reports_no_palace_with_python_style_hint() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("missing-palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "search", "GraphQL"])
        .assert()
        .failure()
        .stdout(contains("\"error\": \"No palace found\""))
        .stdout(contains(
            "Run: mempalace init <dir> && mempalace mine <dir>",
        ))
        .stdout(contains("\"palace_path\":"));
}

#[test]
fn cli_search_human_reports_no_palace_with_python_style_text() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("missing-palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "search",
            "GraphQL",
            "--human",
        ])
        .assert()
        .failure()
        .stdout(contains("No palace found at"))
        .stdout(contains(
            "Run: mempalace init <dir> then mempalace mine <dir>",
        ));
}

#[test]
fn cli_search_human_reports_query_errors_with_python_style_text() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
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

    let sqlite = Connection::open(palace.join("palace.sqlite3")).unwrap();
    sqlite
        .execute(
            "UPDATE meta SET value = 'broken-provider' WHERE key = 'embedding_provider'",
            [],
        )
        .unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "search",
            "GraphQL",
            "--human",
        ])
        .assert()
        .failure()
        .stdout(contains("Search error:"))
        .stdout(contains("Palace embedding profile mismatch"));
}

#[test]
fn cli_search_json_reports_query_errors_with_structured_error() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
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

    let sqlite = Connection::open(palace.join("palace.sqlite3")).unwrap();
    sqlite
        .execute(
            "UPDATE meta SET value = 'broken-provider' WHERE key = 'embedding_provider'",
            [],
        )
        .unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "search", "GraphQL"])
        .assert()
        .failure()
        .stdout(contains("\"error\":"))
        .stdout(contains("Search error:"))
        .stdout(contains("Palace embedding profile mismatch"));
}

#[test]
fn cli_mine_dry_run_reports_preview_without_writing_drawers() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("src").join("auth.txt"),
        "JWT authentication dry-run preview.\n\nNothing should be persisted.",
    )
    .unwrap();

    let palace = tmp.path().join("palace");

    let mine_output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let mine: Value = serde_json::from_slice(&mine_output).unwrap();
    assert_eq!(mine["kind"], "mine");
    assert_eq!(mine["mode"], "projects");
    assert_eq!(mine["extract"], "exchange");
    assert_eq!(mine["agent"], "mempalace");
    assert_eq!(mine["configured_rooms"][0], "general");
    assert_eq!(mine["dry_run"], true);
    assert_eq!(mine["files_planned"], 1);
    assert_eq!(mine["files_mined"], 1);
    assert_eq!(mine["respect_gitignore"], true);
    assert_eq!(mine["include_ignored"], Value::Array(vec![]));
    assert_eq!(mine["room_counts"]["general"], 1);
    assert_eq!(
        mine["next_hint"],
        "mempalace search \"what you're looking for\""
    );

    let status_output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "status"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let status: Value = serde_json::from_slice(&status_output).unwrap();
    assert_eq!(status["kind"], "status");
    assert_eq!(status["total_drawers"], 0);
}

#[test]
fn cli_mine_human_prints_python_style_summary() {
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
            "mine",
            project.to_str().unwrap(),
            "--human",
            "--progress",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Mine"))
        .stdout(contains("Wing:"))
        .stdout(contains("Rooms:"))
        .stdout(contains("Files processed: 1"))
        .stdout(contains("Drawers filed:"))
        .stdout(contains("mempalace search"))
        .stderr(contains("auth.txt"));
}

#[test]
fn cli_mine_human_dry_run_reports_preview_only() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("src").join("notes.md"),
        "Dry run should preview mining without writing drawers.",
    )
    .unwrap();
    let palace = tmp.path().join("palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
            "--dry-run",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Mine"))
        .stdout(contains("Mode:            DRY RUN"))
        .stdout(contains("Drawers previewed: 1"))
        .stdout(contains(
            "Persistence:     preview only, no drawers were written",
        ))
        .stdout(contains("Drawers filed:").not());
}

#[test]
fn cli_mine_human_empty_project_reports_no_matching_files() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("target")).unwrap();
    fs::write(
        project.join("target").join("generated.bin"),
        "opaque build artifact",
    )
    .unwrap();
    let palace = tmp.path().join("palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Mine"))
        .stdout(contains("Files:    0"))
        .stdout(contains("Files processed: 0"))
        .stdout(contains("No matching files found."))
        .stdout(contains(
            "Check your project path, ignore rules, and supported file types.",
        ));
}

#[test]
fn cli_mine_progress_prints_to_stderr_while_stdout_stays_json() {
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
            "mine",
            project.to_str().unwrap(),
            "--progress",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"mine\""))
        .stdout(contains("\"files_mined\": 1"))
        .stderr(contains("auth.txt"))
        .stderr(contains("+1"));
}

#[test]
fn cli_mine_dry_run_progress_prints_python_style_preview_to_stderr() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("src").join("notes.md"),
        "Graph search notes.\n\nDry run should not persist drawers.",
    )
    .unwrap();
    let palace = tmp.path().join("palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
            "--dry-run",
            "--progress",
        ])
        .assert()
        .success()
        .stdout(contains("\"dry_run\": true"))
        .stderr(contains("[DRY RUN]"))
        .stderr(contains("room:general"));
}

#[test]
fn cli_mine_rejects_unsupported_convos_mode_with_json_hint() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("chats");
    fs::create_dir_all(&project).unwrap();
    let palace = tmp.path().join("palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
            "--mode",
            "convos",
            "--extract",
            "general",
        ])
        .assert()
        .failure()
        .code(2)
        .stdout(contains("\"error\": \"Unsupported mine mode\""))
        .stdout(contains("\"mode\": \"convos\""))
        .stdout(contains("\"extract\": \"general\""));
}

#[test]
fn cli_mine_human_rejects_unsupported_convos_mode_with_text_hint() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("chats");
    fs::create_dir_all(&project).unwrap();
    let palace = tmp.path().join("palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
            "--mode",
            "convos",
            "--extract",
            "general",
            "--human",
        ])
        .assert()
        .failure()
        .code(2)
        .stdout(contains("MemPalace Mine"))
        .stdout(contains("Mode:     convos"))
        .stdout(contains("Extract:  general"))
        .stdout(contains(
            "Conversation and general extraction are not implemented in Rust yet.",
        ))
        .stdout(contains("Retry with --mode projects"));
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
    assert_eq!(prepare["kind"], "prepare_embedding");
    assert!(
        prepare["sqlite_path"]
            .as_str()
            .unwrap()
            .ends_with("palace.sqlite3")
    );
    assert!(prepare["lance_path"].as_str().unwrap().ends_with("lance"));
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
        .stdout(contains("\"kind\": \"migrate\""))
        .stdout(contains("\"version\":"))
        .stdout(contains("\"schema_version_before\": 1"))
        .stdout(contains("\"schema_version_after\": 4"))
        .stdout(contains("\"changed\": true"));
}

#[test]
fn cli_migrate_human_reports_no_palace_with_python_style_text() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("missing-palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "migrate", "--human"])
        .assert()
        .success()
        .stdout(contains("No palace found at"));
}

#[test]
fn cli_migrate_human_prints_python_style_summary() {
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
        .args(["--palace", palace.to_str().unwrap(), "migrate", "--human"])
        .assert()
        .success()
        .stdout(contains("MemPalace Migrate"))
        .stdout(contains("Palace:"))
        .stdout(contains("SQLite:"))
        .stdout(contains("Before:  1"))
        .stdout(contains("After:   4"))
        .stdout(contains("Migration complete."));
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
        .stdout(contains("\"kind\": \"repair\""))
        .stdout(contains("\"ok\": false"))
        .stdout(contains("SQLite palace file is missing"))
        .stdout(contains("LanceDB directory is missing"));
}

#[test]
fn cli_repair_human_reports_missing_palace_with_python_style_text() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("missing-palace");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "repair", "--human"])
        .assert()
        .success()
        .stdout(contains("No palace found at"));
}

#[test]
fn cli_repair_human_reports_issue_summary_and_next_step() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "repair", "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Repair error:"))
        .stdout(contains("file is not a database"))
        .stdout(contains(
            "Check the palace files, then rerun `mempalace-rs repair`.",
        ));
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
        .stdout(contains("\"kind\": \"repair\""))
        .stdout(contains("\"version\":"))
        .stdout(contains("\"ok\": true"))
        .stdout(contains("\"vector_accessible\": true"))
        .stdout(contains("\"embedding_provider\": \"hash\""))
        .stdout(contains("\"schema_version\": 4"));
}

#[test]
fn cli_repair_human_prints_python_style_diagnostics() {
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
        .args(["--palace", palace.to_str().unwrap(), "repair", "--human"])
        .assert()
        .success()
        .stdout(contains("MemPalace Repair"))
        .stdout(contains("Palace:"))
        .stdout(contains("Drawers found:"))
        .stdout(contains("Schema version: 4"))
        .stdout(contains("Embedding: hash/hash-v1/64"))
        .stdout(contains("Vector access: ok"))
        .stdout(contains("Repair diagnostics look healthy."));
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
    assert_eq!(first["added_by"], "mempalace");
    assert!(first["source_mtime"].as_f64().is_some());
    assert!(first["filed_at"].as_str().is_some());
    assert!(first.get("similarity").is_some());
}

#[test]
fn cli_status_human_prints_python_style_status_blocks() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("docs")).unwrap();
    fs::write(
        project.join("src").join("auth.txt"),
        "JWT authentication tokens are stored locally.\n\nThe team switched to Clerk for auth.",
    )
    .unwrap();
    fs::write(
        project.join("docs").join("guide.md"),
        "Architecture guide for the Rust rewrite.\n\nThis guide explains room taxonomy and project docs.",
    )
    .unwrap();
    fs::write(
        project.join("mempalace.yaml"),
        r#"
wing: my_app
rooms:
  - name: auth
    keywords: [jwt, clerk, token]
  - name: docs
    keywords: [guide, architecture]
"#,
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
        .args(["--palace", palace.to_str().unwrap(), "status", "--human"])
        .assert()
        .success()
        .stdout(contains("MemPalace Status — 2 drawers"))
        .stdout(contains("WING: my_app"))
        .stdout(contains("ROOM: auth"))
        .stdout(contains("ROOM: docs"));
}

#[test]
fn cli_status_human_empty_palace_reports_next_step() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
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
        .args(["--palace", palace.to_str().unwrap(), "status", "--human"])
        .assert()
        .success()
        .stdout(contains("MemPalace Status — 0 drawers"))
        .stdout(contains("Palace is initialized but still empty."))
        .stdout(contains("Run: mempalace mine <dir>"));
}

#[test]
fn cli_search_human_prints_python_style_result_blocks() {
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

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "search",
            "Clerk auth",
            "--wing",
            "my_app",
            "--room",
            "general",
            "--results",
            "3",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("Results for: \"Clerk auth\""))
        .stdout(contains("Wing: my_app"))
        .stdout(contains("Room: general"))
        .stdout(contains("[1] my_app / general"))
        .stdout(contains("Source: auth.txt"))
        .stdout(contains("Match:"))
        .stdout(contains("The team switched to Clerk for auth."));
}

#[test]
fn cli_search_human_reports_no_results_like_python() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
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
            "search",
            "xyzzy_nonexistent_query",
            "--results",
            "1",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains(
            "No results found for: \"xyzzy_nonexistent_query\"",
        ));
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
