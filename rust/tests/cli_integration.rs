use std::fs;

use assert_cmd::Command;
use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::model::DrawerInput;
use mempalace_rs::service::App;
use mempalace_rs::storage::vector::VectorStore;
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
        .stdout(contains("\"project_path\":"))
        .stdout(contains("\"wing\": \"project\""))
        .stdout(contains("\"configured_rooms\":"))
        .stdout(contains("\"config_written\": true"))
        .stdout(contains("\"entities_written\":"))
        .stdout(contains("\"entity_registry_path\":"))
        .stdout(contains("\"entity_registry_written\":"))
        .stdout(contains("\"aaak_entities_path\":"))
        .stdout(contains("\"critical_facts_path\":"))
        .stdout(contains("\"version\":"))
        .stdout(contains("\"schema_version\": 7"))
        .stdout(contains("palace.sqlite3"));

    assert!(project.join("mempalace.yaml").exists());
    assert!(project.join("entity_registry.json").exists());
    assert!(project.join("aaak_entities.md").exists());
    assert!(project.join("critical_facts.md").exists());

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
fn cli_init_writes_entities_json_when_detection_finds_names() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("notes")).unwrap();
    fs::write(
        project.join("notes").join("people.md"),
        "Jordan said Atlas should launch soon.\nJordan wrote the Atlas system notes.",
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
        .stdout(contains("\"entities_written\": true"))
        .stdout(contains("\"entity_registry_written\": true"))
        .stdout(contains("\"aaak_entities_written\": true"))
        .stdout(contains("\"critical_facts_written\": true"))
        .stdout(contains("\"detected_people\":"))
        .stdout(contains("\"detected_projects\":"));

    assert!(project.join("entities.json").exists());
    assert!(project.join("entity_registry.json").exists());
    assert!(project.join("aaak_entities.md").exists());
    assert!(project.join("critical_facts.md").exists());
}

#[test]
fn cli_onboarding_help_mentions_mode_people_and_scan() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["onboarding", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Guide first-run registry and AAAK bootstrap for a project",
        ))
        .stdout(contains("Project directory to seed"))
        .stdout(contains("Usage mode: work, personal, or combo"))
        .stdout(contains("--mode"))
        .stdout(contains("--person"))
        .stdout(contains("--project"))
        .stdout(contains("--alias"))
        .stdout(contains(
            "Comma-separated wing list; defaults follow the selected mode",
        ))
        .stdout(contains("--scan"))
        .stdout(contains("--auto-accept-detected"))
        .stdout(contains("--human"));
}

#[test]
fn cli_onboarding_json_bootstraps_local_world_files() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("world");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("notes.md"),
        "Ever said Lantern should launch soon.\nEver wrote the Lantern architecture notes.",
    )
    .unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "onboarding",
            project.to_str().unwrap(),
            "--mode",
            "combo",
            "--person",
            "Riley,daughter,personal",
            "--person",
            "Ben,co-founder,work",
            "--project",
            "Lantern",
            "--alias",
            "Ry=Riley",
            "--wings",
            "family,work,projects",
            "--scan",
            "--auto-accept-detected",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"onboarding\""))
        .stdout(contains("\"mode\": \"combo\""))
        .stdout(contains("\"wings\":"))
        .stdout(contains("\"aliases\":"))
        .stdout(contains("\"auto_detected_people\":"))
        .stdout(contains("\"entity_registry_path\":"))
        .stdout(contains("\"aaak_entities_path\":"))
        .stdout(contains("\"critical_facts_path\":"));

    assert!(project.join("mempalace.yaml").exists());
    assert!(project.join("entities.json").exists());
    assert!(project.join("entity_registry.json").exists());
    assert!(project.join("aaak_entities.md").exists());
    assert!(project.join("critical_facts.md").exists());

    let registry: Value =
        serde_json::from_str(&fs::read_to_string(project.join("entity_registry.json")).unwrap())
            .unwrap();
    assert_eq!(registry["mode"], "combo");
    assert!(registry["people"]["Riley"].is_object());
    assert!(registry["people"]["Ry"].is_object());
    assert!(
        registry["projects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "Lantern")
    );
}

#[test]
fn cli_onboarding_human_prints_setup_summary() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("world");
    fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "onboarding",
            project.to_str().unwrap(),
            "--mode",
            "work",
            "--person",
            "Ben,co-founder,work",
            "--project",
            "Lantern",
            "--wings",
            "projects,decisions,research",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Onboarding"))
        .stdout(contains("Mode:    work"))
        .stdout(contains("Wings:   projects, decisions, research"))
        .stdout(contains("Registry:"))
        .stdout(contains("Your local world bootstrap is ready."));
}

#[test]
fn cli_registry_summary_lookup_learn_and_research_work() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("notes.md"),
        "Ever said Atlas should launch soon.\nEver wrote the Atlas architecture notes.",
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
        .args(["registry", "summary", project.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"registry_summary\""))
        .stdout(contains("\"people_count\":"))
        .stdout(contains("\"projects\":"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "registry",
            "lookup",
            project.to_str().unwrap(),
            "Ever",
            "--context",
            "Have you ever seen this before?",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"registry_lookup\""))
        .stdout(contains("\"type\": \"concept\""));

    fs::write(
        project.join("more.md"),
        "Riley said Lantern shipped.\nRiley wrote the Lantern deploy notes.",
    )
    .unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "learn", project.to_str().unwrap(), "--human"])
        .assert()
        .success()
        .stdout(contains("MemPalace Registry Learn"))
        .stdout(contains("Riley"))
        .stdout(contains("Lantern"));

    let registry_path = project.join("entity_registry.json");
    let mut registry = mempalace_rs::registry::EntityRegistry::load(&registry_path).unwrap();
    registry.wiki_cache.insert(
        "Max".to_string(),
        mempalace_rs::registry::RegistryResearchEntry {
            word: "Max".to_string(),
            inferred_type: "person".to_string(),
            confidence: 0.9,
            wiki_summary: Some("max is a given name".to_string()),
            wiki_title: Some("Max".to_string()),
            note: None,
            confirmed: false,
            confirmed_type: None,
        },
    );
    registry.save(&registry_path).unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "research", project.to_str().unwrap(), "Max"])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"registry_research\""))
        .stdout(contains("\"word\": \"Max\""));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "registry",
            "confirm",
            project.to_str().unwrap(),
            "Max",
            "--type",
            "person",
            "--relationship",
            "coworker",
            "--context",
            "work",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Registry Confirm"))
        .stdout(contains("Max"))
        .stdout(contains("coworker"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "registry",
            "query",
            project.to_str().unwrap(),
            "Max said Lantern should ship with Riley",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Registry Query"))
        .stdout(contains("Max"))
        .stdout(contains("Riley"));
}

#[test]
fn cli_compress_json_stores_aaak_summaries() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("src").join("auth.txt"),
        "Alice decided to switch authentication to Clerk because the GraphQL auth flow kept failing deploys.",
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

    let output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "compress"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(summary["kind"], "compress");
    assert_eq!(summary["processed"], 1);
    assert_eq!(summary["stored"], 1);

    let stored: i64 = Connection::open(palace.join("palace.sqlite3"))
        .unwrap()
        .query_row("SELECT COUNT(*) FROM compressed_drawers", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(stored, 1);
}

#[test]
fn cli_wake_up_human_prints_identity_and_layer1() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("src").join("auth.txt"),
        "Alice decided to switch authentication to Clerk because the GraphQL auth flow kept failing deploys.",
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
    fs::write(
        palace.join("identity.txt"),
        "## L0 — IDENTITY\nI am Atlas, a local-first memory assistant.",
    )
    .unwrap();
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
        .args(["--palace", palace.to_str().unwrap(), "wake-up", "--human"])
        .assert()
        .success()
        .stdout(contains("I am Atlas"))
        .stdout(contains("ESSENTIAL STORY"))
        .stdout(contains("auth.txt"));
}

#[test]
fn cli_hook_session_start_outputs_empty_json_and_initializes_state() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace");

    let output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "hook",
            "run",
            "--hook",
            "session-start",
            "--harness",
            "codex",
        ])
        .write_stdin(r#"{"session_id":"abc-123","stop_hook_active":false}"#)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let payload: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(payload, serde_json::json!({}));
    assert!(palace.join("hook_state").join("hook.log").exists());
}

#[test]
fn cli_hook_stop_blocks_after_15_messages() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace");
    let transcript = tmp.path().join("transcript.jsonl");
    let mut lines = String::new();
    for idx in 0..15 {
        lines.push_str(&format!(
            "{{\"message\":{{\"role\":\"user\",\"content\":\"message {idx}\"}}}}\n"
        ));
    }
    fs::write(&transcript, lines).unwrap();

    let output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "hook",
            "run",
            "--hook",
            "stop",
            "--harness",
            "claude-code",
        ])
        .write_stdin(format!(
            r#"{{"session_id":"save-me","stop_hook_active":false,"transcript_path":"{}"}}"#,
            transcript.display()
        ))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let payload: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(payload["decision"], "block");
    assert!(
        payload["reason"]
            .as_str()
            .unwrap()
            .contains("AUTO-SAVE checkpoint")
    );
}

#[test]
fn cli_hook_stop_passes_through_when_already_active() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace");

    let output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "hook",
            "run",
            "--hook",
            "stop",
            "--harness",
            "codex",
        ])
        .write_stdin(
            r#"{"session_id":"active","stop_hook_active":true,"transcript_path":"/tmp/missing"}"#,
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let payload: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(payload, serde_json::json!({}));
}

#[test]
fn cli_hook_precompact_always_blocks() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace");

    let output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "hook",
            "run",
            "--hook",
            "precompact",
            "--harness",
            "codex",
        ])
        .write_stdin(r#"{"session_id":"compact-me"}"#)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let payload: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(payload["decision"], "block");
    assert!(
        payload["reason"]
            .as_str()
            .unwrap()
            .contains("COMPACTION IMMINENT")
    );
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
        .stdout(contains("project bootstrap"))
        .stdout(contains("--palace <PALACE>"))
        .stdout(contains("Where the palace lives"))
        .stdout(contains("--hf-endpoint <HF_ENDPOINT>"))
        .stdout(contains(
            "Override the HuggingFace endpoint used by fastembed model downloads",
        ))
        .stdout(contains("mempalace-rs mine ~/projects/my_app"))
        .stdout(contains(
            "mempalace-rs onboarding ~/projects/my_app --mode combo --scan",
        ))
        .stdout(contains(
            "mempalace-rs normalize ~/exports/chat.jsonl --human",
        ))
        .stdout(contains(
            "mempalace-rs recall --wing my_app --room decisions",
        ))
        .stdout(contains("mempalace-rs registry summary ~/projects/my_app"))
        .stdout(contains("onboarding"))
        .stdout(contains("normalize"))
        .stdout(contains("recall"))
        .stdout(contains("registry"))
        .stdout(contains("hook"))
        .stdout(contains("instructions"))
        .stdout(contains("mcp"))
        .stdout(contains("migrate"))
        .stdout(contains("repair"));
}

#[test]
fn cli_hook_help_mentions_stdio_behavior() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["hook", "--help"])
        .assert()
        .success()
        .stdout(contains("reads JSON from stdin, outputs JSON to stdout"))
        .stdout(contains("run"))
        .stdout(contains("Execute a hook"));
}

#[test]
fn cli_hook_run_help_mentions_hook_name_and_harness() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["hook", "run", "--help"])
        .assert()
        .success()
        .stdout(contains("Execute a hook"))
        .stdout(contains("Hook name to run"))
        .stdout(contains("Harness type"));
}

#[test]
fn cli_instructions_help_outputs_markdown() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["instructions", "help"])
        .assert()
        .success()
        .stdout(contains("# MemPalace"))
        .stdout(contains("mempalace-rs hook run"));
}

#[test]
fn cli_recall_help_mentions_wing_room_and_results() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["recall", "--help"])
        .assert()
        .success()
        .stdout(contains("Limit recall to one project/wing"))
        .stdout(contains("--wing"))
        .stdout(contains("Limit recall to one room"))
        .stdout(contains("--room"))
        .stdout(contains("Maximum number of drawers to return"))
        .stdout(contains("--results"))
        .stdout(contains("Recall stored drawers by wing/room"))
        .stdout(contains("human-readable recall output"));
}

#[test]
fn cli_layers_status_help_mentions_layer_stack() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["layers-status", "--help"])
        .assert()
        .success()
        .stdout(contains("Show Layer 0-3 stack status"))
        .stdout(contains("--human"))
        .stdout(contains("human-readable layer status"));
}

#[test]
fn cli_mcp_help_mentions_setup_and_serve_flags() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["mcp", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Show MCP setup help or run the read-only MCP server",
        ))
        .stdout(contains("--setup"))
        .stdout(contains("--serve"))
        .stdout(contains("Print Python-style MCP setup instructions"))
        .stdout(contains(
            "Run the MCP server on stdio instead of printing setup help",
        ))
        .stdout(contains("read-only MCP server"));
}

#[test]
fn cli_recall_human_prints_layer2_style_output() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("notes")).unwrap();
    fs::write(
        project.join("notes").join("billing.md"),
        "Billing migration note.\n\nWe moved invoices into Stripe.",
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
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "recall",
            "--human",
            "--results",
            "5",
        ])
        .assert()
        .success()
        .stdout(contains("## L2"))
        .stdout(contains("Billing migration note"))
        .stdout(contains("billing.md"));
}

#[test]
fn cli_layers_status_human_prints_stack_summary() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(
        palace.join("identity.txt"),
        "I am Atlas, a personal AI assistant for Alice.",
    )
    .unwrap();

    let project = tmp.path().join("project");
    fs::create_dir_all(project.join("notes")).unwrap();
    fs::write(
        project.join("notes").join("arch.md"),
        "Architecture discussion.\n\nWe chose Rust for local-first memory.",
    )
    .unwrap();

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
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "layers-status",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Layer Status"))
        .stdout(contains("L0 tokens:"))
        .stdout(contains("L1:"))
        .stdout(contains("L2:"))
        .stdout(contains("L3:"));
}

#[test]
fn cli_mcp_default_prints_python_style_quick_setup() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace path");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "mcp"])
        .assert()
        .success()
        .stdout(contains("MemPalace MCP quick setup:"))
        .stdout(contains("claude mcp add mempalace --"))
        .stdout(contains("Run the server directly:"))
        .stdout(contains("Optional custom palace:"))
        .stdout(contains("mempalace-rs mcp --serve --palace"));
}

#[test]
fn cli_mcp_setup_flag_still_prints_python_style_quick_setup() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace path");

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "mcp", "--setup"])
        .assert()
        .success()
        .stdout(contains("MemPalace MCP quick setup:"))
        .stdout(contains("Run the server directly:"))
        .stdout(contains("mempalace-rs mcp --serve --palace"));
}

#[test]
fn cli_registry_help_mentions_summary_lookup_learn_and_research() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Inspect and update the project-local entity registry",
        ))
        .stdout(contains("summary"))
        .stdout(contains("lookup"))
        .stdout(contains("learn"))
        .stdout(contains("add-person"))
        .stdout(contains("add-project"))
        .stdout(contains("add-alias"))
        .stdout(contains("query"))
        .stdout(contains("research"))
        .stdout(contains("confirm"));
}

#[test]
fn cli_registry_read_subcommands_help_cover_paths_and_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "summary", "--help"])
        .assert()
        .success()
        .stdout(contains("Show a summary of entity_registry.json"))
        .stdout(contains(
            "Project directory containing entity_registry.json",
        ))
        .stdout(contains("human-readable registry summary"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "lookup", "--help"])
        .assert()
        .success()
        .stdout(contains("Look up one word in entity_registry.json"))
        .stdout(contains("Word to look up"))
        .stdout(contains(
            "Context sentence used for ambiguous-name disambiguation",
        ))
        .stdout(contains("human-readable lookup output"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "learn", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Learn new people/projects from local files into entity_registry.json",
        ))
        .stdout(contains("Project directory to scan and update"))
        .stdout(contains("human-readable learn summary"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "query", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Extract known people and unknown capitalized candidates from a query",
        ))
        .stdout(contains("Query text to inspect"))
        .stdout(contains("human-readable query summary"));
}

#[test]
fn cli_registry_write_subcommands_help_cover_entity_fields() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "add-person", "--help"])
        .assert()
        .success()
        .stdout(contains("Add one person to entity_registry.json"))
        .stdout(contains("Person name to add"))
        .stdout(contains("Relationship or role"))
        .stdout(contains("Context bucket: work or personal"))
        .stdout(contains("human-readable write summary"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "add-project", "--help"])
        .assert()
        .success()
        .stdout(contains("Add one project to entity_registry.json"))
        .stdout(contains("Project name to add"))
        .stdout(contains("human-readable write summary"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "add-alias", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Add an alias for an existing person in entity_registry.json",
        ))
        .stdout(contains("Existing canonical person name"))
        .stdout(contains("Alias or nickname to add"))
        .stdout(contains("human-readable write summary"));
}

#[test]
fn cli_registry_research_subcommands_help_cover_confirmation_fields() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "research", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Research one unknown word into the registry wiki cache",
        ))
        .stdout(contains("Word to research via Wikipedia"))
        .stdout(contains(
            "Mark the researched result as confirmed immediately",
        ))
        .stdout(contains("human-readable research summary"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["registry", "confirm", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Confirm one researched word and promote it into the registry",
        ))
        .stdout(contains("Word already present in the wiki cache"))
        .stdout(contains("Confirmed entity type, usually person"))
        .stdout(contains("Relationship or role if confirming a person"))
        .stdout(contains("Context bucket: work or personal"))
        .stdout(contains("human-readable confirm summary"));
}

#[test]
fn cli_compress_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["compress", "--help"])
        .assert()
        .success()
        .stdout(contains("Compress drawers into AAAK summaries"))
        .stdout(contains("Limit compression to one project/wing"))
        .stdout(contains("Preview AAAK summaries without storing them"))
        .stdout(contains("human-readable compression summary"));
}

#[test]
fn cli_wake_up_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["wake-up", "--help"])
        .assert()
        .success()
        .stdout(contains("Show L0 + L1 wake-up context"))
        .stdout(contains("Show wake-up context for one project/wing"))
        .stdout(contains("human-readable wake-up context"));
}

#[test]
fn cli_init_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(contains("Set up a palace directory for a project"))
        .stdout(contains("Project directory to set up"))
        .stdout(contains("Auto-accept detected bootstrap files"))
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
        .stdout(contains("Warm the embedding model during the doctor run"))
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
        .stdout(contains("How many warm-up attempts to make"))
        .stdout(contains("Milliseconds to wait between attempts"))
        .stdout(contains("human-readable prepare summary"));
}

#[test]
fn cli_mine_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["mine", "--help"])
        .assert()
        .success()
        .stdout(contains("human-readable mine summary"))
        .stdout(contains("--include-ignored"))
        .stdout(contains("--no-gitignore"))
        .stdout(contains("--progress"));
}

#[test]
fn cli_status_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["status", "--help"])
        .assert()
        .success()
        .stdout(contains("Show what has been filed in the palace"))
        .stdout(contains("Usage: mempalace-rs status"))
        .stdout(contains("human-readable palace status"));
}

#[test]
fn cli_repair_help_mentions_human_output() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["repair", "--help"])
        .assert()
        .success()
        .stdout(contains("Run repair diagnostics or repair subcommands"))
        .stdout(contains("human-readable repair diagnostics"))
        .stdout(contains("scan"))
        .stdout(contains("write corrupt_ids.txt"))
        .stdout(contains("prune"))
        .stdout(contains("Delete IDs listed in corrupt_ids.txt"))
        .stdout(contains("rebuild"))
        .stdout(contains("Rebuild the vector store from SQLite drawers"));
}

#[test]
fn cli_repair_scan_help_mentions_wing_filter() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["repair", "scan", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Scan for vector/SQLite drift and write corrupt_ids.txt",
        ))
        .stdout(contains("Scan only this wing"));
}

#[test]
fn cli_repair_prune_help_mentions_confirm_flag() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["repair", "prune", "--help"])
        .assert()
        .success()
        .stdout(contains("Delete IDs listed in corrupt_ids.txt"))
        .stdout(contains("Actually delete the queued IDs"));
}

#[test]
fn cli_repair_rebuild_help_mentions_vector_rebuild() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["repair", "rebuild", "--help"])
        .assert()
        .success()
        .stdout(contains("Rebuild the vector store from SQLite drawers"));
}

#[test]
fn cli_dedup_help_mentions_threshold_and_stats() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["dedup", "--help"])
        .assert()
        .success()
        .stdout(contains("Deduplicate near-identical drawers"))
        .stdout(contains("Cosine distance threshold"))
        .stdout(contains("Preview without deleting"))
        .stdout(contains("Show stats only"))
        .stdout(contains("Scope dedup to one wing"))
        .stdout(contains("Filter by source file pattern"));
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
        .stdout(contains("What to search for"))
        .stdout(contains("Limit to one project/wing"))
        .stdout(contains("Number of results"))
        .stdout(contains("human-readable search output"));
}

#[test]
fn cli_split_help_mentions_transcript_megafiles() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["split", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Split concatenated transcript mega-files into per-session files",
        ))
        .stdout(contains("Directory containing transcript files"))
        .stdout(contains("Write split files here"))
        .stdout(contains("Only split files containing at least N sessions"))
        .stdout(contains("Show what would be split without writing files"));
}

#[test]
fn cli_normalize_help_mentions_chat_export_normalization() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["normalize", "--help"])
        .assert()
        .success()
        .stdout(contains(
            "Normalize one chat export into MemPalace transcript format",
        ))
        .stdout(contains("Chat export or transcript file to normalize"))
        .stdout(contains("Print human-readable preview instead of JSON"))
        .stdout(contains("human-readable preview"));
}

#[test]
fn cli_normalize_json_reports_changed_transcript() {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("session.jsonl");
    fs::write(
        &source,
        r#"{"type":"session_meta","payload":{"id":"demo"}}
{"type":"event_msg","payload":{"type":"user_message","message":"Riley knoe the deploy befor lunch"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"We fixed it."}}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("entity_registry.json"),
        r#"{
  "version": 1,
  "mode": "work",
  "people": {
    "Riley": {
      "source": "manual",
      "contexts": ["work"],
      "aliases": [],
      "relationship": "coworker",
      "confidence": 1.0
    }
  },
  "projects": [],
  "ambiguous_flags": [],
  "wiki_cache": {}
}"#,
    )
    .unwrap();

    let output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["normalize", source.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(summary["kind"], "normalize");
    assert_eq!(summary["changed"], true);
    assert!(
        summary["normalized"]
            .as_str()
            .unwrap()
            .contains("> Riley know the deploy before lunch")
    );
}

#[test]
fn cli_normalize_human_prints_preview() {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("already.txt");
    fs::write(&source, "> user turn\nassistant turn\n").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["normalize", source.to_str().unwrap(), "--human"])
        .assert()
        .success()
        .stdout(contains("MemPalace Normalize"))
        .stdout(contains("Preview"))
        .stdout(contains("> user turn"));
}

#[test]
fn cli_split_dry_run_reports_output_without_writing() {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("transcripts");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("mega.txt"),
        concat!(
            "Claude Code v1\n",
            "⏺ 9:30 AM Monday, March 30, 2026\n",
            "> first prompt about pricing migration\n",
            "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n",
            "Claude Code v1\n",
            "⏺ 10:45 AM Monday, March 30, 2026\n",
            "> second prompt about tunnel graph\n",
            "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n",
        ),
    )
    .unwrap();

    let output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["split", source.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(summary["kind"], "split");
    assert_eq!(summary["dry_run"], true);
    assert_eq!(summary["mega_files"], 1);
    assert_eq!(summary["files_created"], 2);
    assert!(source.join("mega.txt").exists());
    assert!(!source.join("mega.mega_backup").exists());
}

#[test]
fn cli_split_writes_files_and_renames_backup() {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("transcripts");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join("mega.txt"),
        concat!(
            "Claude Code v1\n",
            "⏺ 9:30 AM Monday, March 30, 2026\n",
            "> first prompt about pricing migration\n",
            "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n",
            "Claude Code v1\n",
            "⏺ 10:45 AM Monday, March 30, 2026\n",
            "> second prompt about tunnel graph\n",
            "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\n",
        ),
    )
    .unwrap();

    let output = Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["split", source.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(summary["files_created"], 2);
    assert!(source.join("mega.mega_backup").exists());
    assert!(!source.join("mega.txt").exists());
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
        .stdout(contains("Project:"))
        .stdout(contains("Wing:"))
        .stdout(contains("Palace:"))
        .stdout(contains("SQLite:"))
        .stdout(contains("LanceDB:"))
        .stdout(contains("Schema:  7"))
        .stdout(contains("Config:"))
        .stdout(contains("Registry:"))
        .stdout(contains("AAAK:"))
        .stdout(contains("Facts:"))
        .stdout(contains("Rooms:"))
        .stdout(contains("Palace initialized."));
}

#[test]
fn cli_init_human_reports_broken_sqlite_with_text_error() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let palace = tmp.path().join("palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "init",
            project.to_str().unwrap(),
            "--human",
        ])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Init error:"))
        .stdout(contains("file is not a database"))
        .stdout(contains(
            "Check the palace path and SQLite file, then rerun `mempalace-rs init <dir>`.",
        ));
}

#[test]
fn cli_init_reports_broken_sqlite_with_structured_error() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let palace = tmp.path().join("palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "init",
            project.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Init error:"))
        .stdout(contains("file is not a database"));
}

#[test]
fn cli_init_reports_invalid_provider_with_structured_error() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["init", project.to_str().unwrap()])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Init error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains("\"hint\":"))
        .stdout(contains("rerun `mempalace-rs init <dir>`"));
}

#[test]
fn cli_init_human_reports_invalid_provider_with_text_error() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["init", project.to_str().unwrap(), "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Init error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains(
            "Check the palace path and SQLite file, then rerun `mempalace-rs init <dir>`.",
        ));
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
fn cli_doctor_human_reports_invalid_provider_with_text_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["doctor", "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Doctor error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains(
            "Check the embedding provider and local runtime, then rerun `mempalace-rs doctor`.",
        ));
}

#[test]
fn cli_doctor_reports_invalid_provider_with_structured_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["doctor"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Doctor error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains("\"hint\":"))
        .stdout(contains("rerun `mempalace-rs doctor`"));
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
fn cli_prepare_embedding_human_reports_invalid_provider_with_text_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args([
            "prepare-embedding",
            "--attempts",
            "1",
            "--wait-ms",
            "0",
            "--human",
        ])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Prepare embedding error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains(
            "Check the palace files and embedding runtime, then rerun `mempalace-rs prepare-embedding`.",
        ));
}

#[test]
fn cli_prepare_embedding_reports_invalid_provider_with_structured_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["prepare-embedding", "--attempts", "1", "--wait-ms", "0"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Prepare embedding error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains("\"hint\":"))
        .stdout(contains("rerun `mempalace-rs prepare-embedding`"));
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
fn cli_status_reports_invalid_provider_with_structured_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["status"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Status error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains("\"hint\":"))
        .stdout(contains("rerun `mempalace-rs status`"));
}

#[test]
fn cli_status_human_reports_invalid_provider_with_text_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["status", "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Status error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains(
            "Check the palace files, then rerun `mempalace-rs status`.",
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
fn cli_search_human_reports_invalid_provider_with_text_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["search", "GraphQL", "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Search error:"))
        .stdout(contains("Unsupported embedding provider: broken"));
}

#[test]
fn cli_search_reports_invalid_provider_with_structured_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["search", "GraphQL"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Search error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains("\"hint\":"))
        .stdout(contains("rerun `mempalace-rs search <query>`"));
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
fn cli_search_reports_broken_sqlite_with_structured_error() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "search", "GraphQL"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Search error:"))
        .stdout(contains("file is not a database"));
}

#[test]
fn cli_search_human_reports_broken_sqlite_with_text_error() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

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
        .code(1)
        .stdout(contains("Search error:"))
        .stdout(contains("file is not a database"));
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
fn cli_mine_reports_broken_sqlite_with_structured_error() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Mine error:"))
        .stdout(contains("file is not a database"));
}

#[test]
fn cli_mine_human_reports_broken_sqlite_with_text_error() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "mine",
            project.to_str().unwrap(),
            "--human",
        ])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Mine error:"))
        .stdout(contains("file is not a database"))
        .stdout(contains(
            "Check the embedding provider and project path, then rerun `mempalace-rs mine <dir>`.",
        ));
}

#[test]
fn cli_mine_human_reports_invalid_provider_with_text_error() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["mine", project.to_str().unwrap(), "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Mine error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains(
            "Check the embedding provider and project path, then rerun `mempalace-rs mine <dir>`.",
        ));
}

#[test]
fn cli_mine_reports_invalid_provider_with_structured_error() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["mine", project.to_str().unwrap()])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Mine error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains("\"hint\":"))
        .stdout(contains("rerun `mempalace-rs mine <dir>`"));
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
        .stdout(contains("Run mode:        DRY RUN"))
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
fn cli_mine_convos_general_empty_dir_returns_empty_summary_json() {
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
        .success()
        .stdout(contains("\"mode\": \"convos\""))
        .stdout(contains("\"extract\": \"general\""))
        .stdout(contains("\"files_planned\": 0"))
        .stdout(contains("\"files_processed\": 0"));
}

#[test]
fn cli_mine_convos_general_empty_dir_returns_empty_summary_human() {
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
        .success()
        .stdout(contains("MemPalace Mine"))
        .stdout(contains("Mode:     convos"))
        .stdout(contains("Extract:  general"))
        .stdout(contains("Files processed: 0"))
        .stdout(contains("No matching files found."));
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
        .stdout(contains("\"schema_version_after\": 7"))
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
fn cli_migrate_reports_invalid_provider_with_structured_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["migrate"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Migrate error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains("\"hint\":"))
        .stdout(contains("rerun `mempalace-rs migrate`"));
}

#[test]
fn cli_migrate_human_reports_invalid_provider_with_text_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["migrate", "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Migrate error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains(
            "Check the palace SQLite file, then rerun `mempalace-rs migrate`.",
        ));
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
        .stdout(contains("After:   7"))
        .stdout(contains("Migration complete."));
}

#[test]
fn cli_migrate_human_reports_broken_sqlite_with_text_error() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "migrate", "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Migrate error:"))
        .stdout(contains("file is not a database"))
        .stdout(contains(
            "Check the palace SQLite file, then rerun `mempalace-rs migrate`.",
        ));
}

#[test]
fn cli_migrate_reports_broken_sqlite_with_structured_error() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "migrate"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Migrate error:"))
        .stdout(contains("file is not a database"));
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
fn cli_repair_reports_invalid_provider_with_structured_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["repair"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Repair error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains("\"hint\":"))
        .stdout(contains("rerun `mempalace-rs repair`"));
}

#[test]
fn cli_repair_human_reports_invalid_provider_with_text_error() {
    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "broken")
        .args(["repair", "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Repair error:"))
        .stdout(contains("Unsupported embedding provider: broken"))
        .stdout(contains(
            "Check the palace files, then rerun `mempalace-rs repair`.",
        ));
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
fn cli_repair_reports_broken_sqlite_with_structured_error() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "repair"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Repair error:"))
        .stdout(contains("file is not a database"));
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
        .stdout(contains("\"schema_version\": 7"));
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
        .stdout(contains("Schema version: 7"))
        .stdout(contains("Embedding: hash/hash-v1/64"))
        .stdout(contains("Vector access: ok"))
        .stdout(contains("Repair diagnostics look healthy."));
}

#[test]
fn cli_repair_scan_and_rebuild_cover_vector_drift() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let drawer_id = runtime.block_on(async {
        let mut config = AppConfig::resolve(Some(&palace)).unwrap();
        config.embedding.backend = EmbeddingBackend::Hash;
        let app = App::new(config.clone()).unwrap();
        app.init().await.unwrap();
        let result = app
            .add_drawer(
                "project",
                "general",
                "This is a stable project note for repair testing.",
                Some("repair.txt"),
                Some("codex"),
            )
            .await
            .unwrap();
        let vector = VectorStore::connect(&config.lance_path()).await.unwrap();
        vector.delete_drawer(64, &result.drawer_id).await.unwrap();
        vector
            .add_drawers(
                &[DrawerInput {
                    id: "orphan_drawer".to_string(),
                    wing: "project".to_string(),
                    room: "general".to_string(),
                    source_file: "orphan.txt".to_string(),
                    source_path: "orphan.txt".to_string(),
                    source_hash: "orphan".to_string(),
                    source_mtime: None,
                    chunk_index: 0,
                    added_by: "codex".to_string(),
                    filed_at: "2026-04-18T00:00:00Z".to_string(),
                    ingest_mode: "mcp".to_string(),
                    extract_mode: "manual".to_string(),
                    text: "orphan drawer".to_string(),
                }],
                &[vec![1.0; 64]],
            )
            .await
            .unwrap();
        result.drawer_id
    });

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "repair", "scan"])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"repair_scan\""))
        .stdout(contains("orphan_drawer"))
        .stdout(contains(&drawer_id));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "repair",
            "prune",
            "--confirm",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"repair_prune\""))
        .stdout(contains("\"deleted_from_vector\": 1"));

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args(["--palace", palace.to_str().unwrap(), "repair", "rebuild"])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"repair_rebuild\""))
        .stdout(contains("\"rebuilt\": 1"));
}

#[test]
fn cli_dedup_human_prints_summary() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("palace");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let mut config = AppConfig::resolve(Some(&palace)).unwrap();
        config.embedding.backend = EmbeddingBackend::Hash;
        let app = App::new(config.clone()).unwrap();
        app.init().await.unwrap();
        let sqlite =
            mempalace_rs::storage::sqlite::SqliteStore::open(&config.sqlite_path()).unwrap();
        let drawer_a = DrawerInput {
            id: "dup_a".to_string(),
            wing: "project".to_string(),
            room: "general".to_string(),
            source_file: "dup.txt".to_string(),
            source_path: "dup.txt".to_string(),
            source_hash: "a".to_string(),
            source_mtime: None,
            chunk_index: 0,
            added_by: "codex".to_string(),
            filed_at: "2026-04-18T00:00:00Z".to_string(),
            ingest_mode: "projects".to_string(),
            extract_mode: "exchange".to_string(),
            text: "The deployment fix was to update the server config and rerun tests.".to_string(),
        };
        let drawer_b = DrawerInput {
            id: "dup_b".to_string(),
            chunk_index: 1,
            source_hash: "b".to_string(),
            text: "The deployment fix was to update the server config and rerun all tests."
                .to_string(),
            ..drawer_a.clone()
        };
        sqlite.insert_drawer(&drawer_a).unwrap();
        sqlite.insert_drawer(&drawer_b).unwrap();
        let vector = VectorStore::connect(&config.lance_path()).await.unwrap();
        vector
            .add_drawers(&[drawer_a, drawer_b], &[vec![1.0; 64], vec![1.0; 64]])
            .await
            .unwrap();
    });

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .env("MEMPALACE_RS_EMBED_PROVIDER", "hash")
        .args([
            "--palace",
            palace.to_str().unwrap(),
            "dedup",
            "--source",
            "dup.txt",
            "--dry-run",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Deduplicator"))
        .stdout(contains("Source filter: dup.txt"))
        .stdout(contains("[DRY RUN] No changes written"));
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
fn cli_status_human_reports_broken_sqlite_with_text_error() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "status", "--human"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("Status error:"))
        .stdout(contains("file is not a database"))
        .stdout(contains(
            "Check the palace files, then rerun `mempalace-rs status`.",
        ));
}

#[test]
fn cli_status_reports_broken_sqlite_with_structured_error() {
    let tmp = tempdir().unwrap();
    let palace = tmp.path().join("broken-palace");
    fs::create_dir_all(&palace).unwrap();
    fs::write(palace.join("palace.sqlite3"), "not a sqlite database").unwrap();

    Command::cargo_bin("mempalace-rs")
        .unwrap()
        .args(["--palace", palace.to_str().unwrap(), "status"])
        .assert()
        .failure()
        .code(1)
        .stdout(contains("\"error\":"))
        .stdout(contains("Status error:"))
        .stdout(contains("file is not a database"));
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
            "--results",
            "3",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("Results for: \"Clerk auth\""))
        .stdout(contains("Wing: my_app"))
        .stdout(contains("[1] my_app /"))
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

#[test]
fn cli_mine_convos_exchange_smoke() {
    let tmp = tempdir().unwrap();
    let convo_dir = tmp.path().join("convos");
    fs::create_dir_all(&convo_dir).unwrap();
    fs::write(
        convo_dir.join("chat.txt"),
        "> why did the deploy fail?\nThe deploy failed because the server config was broken.\n\n> how did we fix it?\nWe fixed the server config and reran the deploy.\n",
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
            convo_dir.to_str().unwrap(),
            "--mode",
            "convos",
        ])
        .assert()
        .success()
        .stdout(contains("\"mode\": \"convos\""))
        .stdout(contains("\"extract\": \"exchange\""))
        .stdout(contains("\"files_mined\": 1"))
        .stdout(contains("\"drawers_added\": 2"))
        .stdout(contains("\"room_counts\":"));
}

#[test]
fn cli_mine_convos_general_smoke() {
    let tmp = tempdir().unwrap();
    let convo_dir = tmp.path().join("convos");
    fs::create_dir_all(&convo_dir).unwrap();
    fs::write(
        convo_dir.join("memories.md"),
        "We decided to use LanceDB because the local-first trade-off is better.\n\nI prefer explicit APIs.\n\nThe migration problem was painful, but we fixed it and now it works.\n\nI feel proud and grateful that the rewrite finally feels stable.\n",
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
            convo_dir.to_str().unwrap(),
            "--mode",
            "convos",
            "--extract",
            "general",
        ])
        .assert()
        .success()
        .stdout(contains("\"mode\": \"convos\""))
        .stdout(contains("\"extract\": \"general\""))
        .stdout(contains("\"decision\":"))
        .stdout(contains("\"milestone\":"))
        .stdout(contains("\"emotional\":"));
}

#[test]
fn cli_mine_convos_dry_run_reports_room_counts() {
    let tmp = tempdir().unwrap();
    let convo_dir = tmp.path().join("convos");
    fs::create_dir_all(&convo_dir).unwrap();
    fs::write(
        convo_dir.join("chat.txt"),
        "Human: why did the deploy fail?\nAssistant: The server config was broken.\nHuman: what fixed it?\nAssistant: We updated the deploy config and reran tests.\n",
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
            convo_dir.to_str().unwrap(),
            "--mode",
            "convos",
            "--dry-run",
            "--progress",
        ])
        .assert()
        .success()
        .stdout(contains("\"dry_run\": true"))
        .stdout(contains("\"room_counts\":"))
        .stderr(contains("[DRY RUN] chat.txt -> room:technical"));
}

#[test]
fn cli_mine_convos_human_prints_python_style_summary() {
    let tmp = tempdir().unwrap();
    let convo_dir = tmp.path().join("convos");
    fs::create_dir_all(&convo_dir).unwrap();
    fs::write(
        convo_dir.join("memories.md"),
        "We decided to use LanceDB because the local-first trade-off is better.\n\nI prefer explicit APIs.\n\nThe migration problem was painful, but we fixed it and now it works.\n",
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
            convo_dir.to_str().unwrap(),
            "--mode",
            "convos",
            "--extract",
            "general",
            "--human",
        ])
        .assert()
        .success()
        .stdout(contains("MemPalace Mine"))
        .stdout(contains("Mode:     convos"))
        .stdout(contains("Extract:  general"))
        .stdout(contains("Files processed: 1"))
        .stdout(contains("Rooms filed:"))
        .stdout(contains("decision"))
        .stdout(contains("milestone"));
}

#[test]
fn cli_mine_convos_exchange_supports_json_chat_export() {
    let tmp = tempdir().unwrap();
    let convo_dir = tmp.path().join("convos");
    fs::create_dir_all(&convo_dir).unwrap();
    fs::write(
        convo_dir.join("chatgpt.json"),
        r#"{
  "mapping": {
    "root": {"id":"root","parent":null,"message":null,"children":["u1"]},
    "u1": {
      "id":"u1",
      "parent":"root",
      "message":{"author":{"role":"user"},"content":{"parts":["Why did the deploy fail?"]}},
      "children":["a1"]
    },
    "a1": {
      "id":"a1",
      "parent":"u1",
      "message":{"author":{"role":"assistant"},"content":{"parts":["The deploy server config was broken."]}},
      "children":["u2"]
    },
    "u2": {
      "id":"u2",
      "parent":"a1",
      "message":{"author":{"role":"user"},"content":{"parts":["How did we fix it?"]}},
      "children":["a2"]
    },
    "a2": {
      "id":"a2",
      "parent":"u2",
      "message":{"author":{"role":"assistant"},"content":{"parts":["We fixed the deploy config and reran tests."]}},
      "children":[]
    }
  }
}"#,
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
            convo_dir.to_str().unwrap(),
            "--mode",
            "convos",
        ])
        .assert()
        .success()
        .stdout(contains("\"files_mined\": 1"))
        .stdout(contains("\"drawers_added\": 2"))
        .stdout(contains("\"technical\":"));
}

#[test]
fn cli_mine_convos_general_progress_summarizes_memory_types() {
    let tmp = tempdir().unwrap();
    let convo_dir = tmp.path().join("convos");
    fs::create_dir_all(&convo_dir).unwrap();
    fs::write(
        convo_dir.join("memories.md"),
        "We decided to use LanceDB because the local-first trade-off is better.\n\nI prefer explicit APIs.\n\nThe migration problem was painful, but we fixed it and now it works.\n\nI feel proud and grateful that the rewrite finally feels stable.\n",
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
            convo_dir.to_str().unwrap(),
            "--mode",
            "convos",
            "--extract",
            "general",
            "--dry-run",
            "--progress",
        ])
        .assert()
        .success()
        .stdout(contains("\"extract\": \"general\""))
        .stderr(contains("[DRY RUN] memories.md ->"))
        .stderr(contains("decision:"))
        .stderr(contains("milestone:"))
        .stderr(contains("emotional:"));
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
