use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::mcp::handle_request;
use mempalace_rs::model::{DrawerInput, KgTriple, MineRequest};
use mempalace_rs::service::App;
use mempalace_rs::storage::sqlite::SqliteStore;
use rusqlite::Connection;
use serde_json::json;
use tempfile::tempdir;

#[tokio::test]
async fn mcp_read_tools_work() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join("notes")).unwrap();
    std::fs::write(
        project.join("notes").join("plan.txt"),
        "Planning notes about GraphQL migration and deployment rollout.",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
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

    let init = handle_request(json!({"method":"initialize","id":1,"params":{}}), &config)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(init["result"]["serverInfo"]["name"], "mempalace");

    let tools = handle_request(json!({"method":"tools/list","id":2,"params":{}}), &config)
        .await
        .unwrap()
        .unwrap();
    let tool_names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert!(tool_names.contains(&"mempalace_status"));
    assert!(tool_names.contains(&"mempalace_search"));
    assert!(tool_names.contains(&"mempalace_check_duplicate"));
    assert!(tool_names.contains(&"mempalace_get_aaak_spec"));
    assert!(tool_names.contains(&"mempalace_wake_up"));
    assert!(tool_names.contains(&"mempalace_recall"));
    assert!(tool_names.contains(&"mempalace_layers_status"));
    assert!(tool_names.contains(&"mempalace_repair"));
    assert!(tool_names.contains(&"mempalace_repair_scan"));
    assert!(tool_names.contains(&"mempalace_repair_prune"));
    assert!(tool_names.contains(&"mempalace_repair_rebuild"));
    assert!(tool_names.contains(&"mempalace_compress"));
    assert!(tool_names.contains(&"mempalace_dedup"));
    assert!(tool_names.contains(&"mempalace_onboarding"));
    assert!(tool_names.contains(&"mempalace_normalize"));
    assert!(tool_names.contains(&"mempalace_split"));
    assert!(tool_names.contains(&"mempalace_instructions"));
    assert!(tool_names.contains(&"mempalace_hook_run"));
    assert!(tool_names.contains(&"mempalace_kg_query"));
    assert!(tool_names.contains(&"mempalace_kg_add"));
    assert!(tool_names.contains(&"mempalace_kg_invalidate"));
    assert!(tool_names.contains(&"mempalace_kg_timeline"));
    assert!(tool_names.contains(&"mempalace_kg_stats"));
    assert!(tool_names.contains(&"mempalace_add_drawer"));
    assert!(tool_names.contains(&"mempalace_delete_drawer"));
    assert!(tool_names.contains(&"mempalace_diary_write"));
    assert!(tool_names.contains(&"mempalace_diary_read"));
    assert!(tool_names.contains(&"mempalace_traverse"));
    assert!(tool_names.contains(&"mempalace_find_tunnels"));
    assert!(tool_names.contains(&"mempalace_graph_stats"));
    assert!(tool_names.contains(&"mempalace_registry_summary"));
    assert!(tool_names.contains(&"mempalace_registry_lookup"));
    assert!(tool_names.contains(&"mempalace_registry_query"));
    assert!(tool_names.contains(&"mempalace_registry_learn"));
    assert!(tool_names.contains(&"mempalace_registry_add_person"));
    assert!(tool_names.contains(&"mempalace_registry_add_project"));
    assert!(tool_names.contains(&"mempalace_registry_add_alias"));
    assert!(tool_names.contains(&"mempalace_registry_research"));
    assert!(tool_names.contains(&"mempalace_registry_confirm"));
    let search_tool = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "mempalace_search")
        .unwrap();
    assert_eq!(
        search_tool["inputSchema"]["required"][0].as_str().unwrap(),
        "query"
    );

    let status = handle_request(
        json!({"method":"tools/call","id":3,"params":{"name":"mempalace_status","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let status_text = status["result"]["content"][0]["text"].as_str().unwrap();
    assert!(status_text.contains("total_drawers"));
    assert!(status_text.contains("\"kind\": \"status\""));
    assert!(status_text.contains("\"sqlite_path\""));
    assert!(status_text.contains("\"lance_path\""));
    assert!(status_text.contains("\"version\""));
    assert!(status_text.contains("\"protocol\""));
    assert!(status_text.contains("\"aaak_dialect\""));

    let search = handle_request(
        json!({"method":"tools/call","id":4,"params":{"name":"mempalace_search","arguments":{"query":"GraphQL","limit":"3"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let search_text = search["result"]["content"][0]["text"].as_str().unwrap();
    assert!(search_text.contains("\"query\": \"GraphQL\""));
    assert!(search_text.contains("\"filters\""));
    assert!(search_text.contains("\"source_file\""));
    assert!(search_text.contains("\"similarity\""));

    let duplicate = handle_request(
        json!({"method":"tools/call","id":5,"params":{"name":"mempalace_check_duplicate","arguments":{"content":"Planning notes about GraphQL migration and deployment rollout.","threshold":"0.8"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let duplicate_text = duplicate["result"]["content"][0]["text"].as_str().unwrap();
    assert!(duplicate_text.contains("\"is_duplicate\": true"));
    assert!(duplicate_text.contains("\"matches\""));
    assert!(duplicate_text.contains("\"id\""));
    assert!(duplicate_text.contains("\"similarity\""));

    let aaak = handle_request(
        json!({"method":"tools/call","id":6,"params":{"name":"mempalace_get_aaak_spec","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let aaak_text = aaak["result"]["content"][0]["text"].as_str().unwrap();
    assert!(aaak_text.contains("\"aaak_spec\""));
    assert!(aaak_text.contains("AAAK is a compressed memory dialect"));

    let wake_up = handle_request(
        json!({"method":"tools/call","id":7,"params":{"name":"mempalace_wake_up","arguments":{"wing":"project"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let wake_up_text = wake_up["result"]["content"][0]["text"].as_str().unwrap();
    assert!(wake_up_text.contains("\"kind\": \"wake_up\""));
    assert!(wake_up_text.contains("\"identity\""));
    assert!(wake_up_text.contains("\"layer1\""));

    let recall = handle_request(
        json!({"method":"tools/call","id":8,"params":{"name":"mempalace_recall","arguments":{"wing":"project","limit":"2"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let recall_text = recall["result"]["content"][0]["text"].as_str().unwrap();
    assert!(recall_text.contains("\"kind\": \"recall\""));
    assert!(recall_text.contains("\"results\""));
    assert!(recall_text.contains("\"total_matches\""));

    let layers_status = handle_request(
        json!({"method":"tools/call","id":9,"params":{"name":"mempalace_layers_status","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let layers_status_text = layers_status["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(layers_status_text.contains("\"kind\": \"layers_status\""));
    assert!(layers_status_text.contains("\"layer0_description\""));
    assert!(layers_status_text.contains("\"layer3_description\""));

    let repair = handle_request(
        json!({"method":"tools/call","id":10,"params":{"name":"mempalace_repair","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let repair_text = repair["result"]["content"][0]["text"].as_str().unwrap();
    assert!(repair_text.contains("\"kind\": \"repair\""));
    assert!(repair_text.contains("\"ok\": true"));

    let repair_scan = handle_request(
        json!({"method":"tools/call","id":11,"params":{"name":"mempalace_repair_scan","arguments":{"wing":"project"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let repair_scan_text = repair_scan["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(repair_scan_text.contains("\"kind\": \"repair_scan\""));
    assert!(repair_scan_text.contains("\"corrupt_ids_path\""));

    let repair_prune = handle_request(
        json!({"method":"tools/call","id":12,"params":{"name":"mempalace_repair_prune","arguments":{"confirm":"false"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let repair_prune_text = repair_prune["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(repair_prune_text.contains("\"kind\": \"repair_prune\""));
    assert!(repair_prune_text.contains("\"confirm\": false"));

    let compress = handle_request(
        json!({"method":"tools/call","id":13,"params":{"name":"mempalace_compress","arguments":{"dry_run":"true","wing":"project"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let compress_text = compress["result"]["content"][0]["text"].as_str().unwrap();
    assert!(compress_text.contains("\"kind\": \"compress\""));
    assert!(compress_text.contains("\"dry_run\": true"));

    for idx in 0..5 {
        app.add_drawer(
            "project",
            "backend",
            &format!(
                "Near-duplicate deployment note for GraphQL rollout and backend migration copy {idx}"
            ),
            Some("notes/dedup.txt"),
            Some("mcp-test"),
        )
        .await
        .unwrap();
    }

    let dedup = handle_request(
        json!({"method":"tools/call","id":14,"params":{"name":"mempalace_dedup","arguments":{"dry_run":"true","threshold":"0.15","min_count":"5","source":"dedup"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let dedup_text = dedup["result"]["content"][0]["text"].as_str().unwrap();
    assert!(dedup_text.contains("\"kind\": \"dedup\""));
    assert!(dedup_text.contains("\"dry_run\": true"));
    assert!(dedup_text.contains("\"sources_checked\""));

    let repair_rebuild = handle_request(
        json!({"method":"tools/call","id":15,"params":{"name":"mempalace_repair_rebuild","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let repair_rebuild_text = repair_rebuild["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(repair_rebuild_text.contains("\"kind\": \"repair_rebuild\""));
    assert!(repair_rebuild_text.contains("\"rebuilt\""));
}

#[tokio::test]
async fn mcp_dedup_defaults_to_dry_run() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    for idx in 0..5 {
        app.add_drawer(
            "project",
            "backend",
            &format!("Near duplicate MCP default safety note about GraphQL rollout copy {idx}"),
            Some("mcp-dedup-default.txt"),
            Some("mcp-test"),
        )
        .await
        .unwrap();
    }

    let dedup = handle_request(
        json!({"method":"tools/call","id":101,"params":{"name":"mempalace_dedup","arguments":{"threshold":"0.15","min_count":"5","source":"mcp-dedup-default"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let dedup_text = dedup["result"]["content"][0]["text"].as_str().unwrap();
    assert!(dedup_text.contains("\"kind\": \"dedup\""));
    assert!(dedup_text.contains("\"dry_run\": true"));
}

#[tokio::test]
async fn mcp_project_bootstrap_tools_work() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("world");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("notes.md"),
        "Ever said Lantern should launch soon.\nEver wrote the Lantern architecture notes.\nEver pushed the Lantern repo.\nhey Ever, should Lantern ship?",
    )
    .unwrap();
    let convo = tmp.path().join("session.jsonl");
    std::fs::write(
        &convo,
        r#"{"type":"session_meta","payload":{"id":"demo"}}
{"type":"event_msg","payload":{"type":"user_message","message":"Riley knoe the deploy befor lunch"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"We fixed it."}}
"#,
    )
    .unwrap();
    std::fs::write(
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

    let transcripts = tmp.path().join("transcripts");
    std::fs::create_dir_all(&transcripts).unwrap();
    std::fs::write(
        transcripts.join("mega.txt"),
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

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    let onboarding = handle_request(
        json!({"method":"tools/call","id":100,"params":{"name":"mempalace_onboarding","arguments":{
            "project_dir": project,
            "mode": "combo",
            "people": ["Riley,daughter,personal", "Ben,co-founder,work"],
            "projects": ["Lantern"],
            "aliases": ["Ry=Riley"],
            "wings": ["family", "work", "projects"],
            "scan": true,
            "auto_accept_detected": true
        }}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let onboarding_text = onboarding["result"]["content"][0]["text"].as_str().unwrap();
    assert!(onboarding_text.contains("\"kind\": \"onboarding\""));
    assert!(onboarding_text.contains("\"mode\": \"combo\""));
    assert!(onboarding_text.contains("\"entity_registry_path\""));

    let normalize = handle_request(
        json!({"method":"tools/call","id":101,"params":{"name":"mempalace_normalize","arguments":{
            "file_path": convo
        }}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let normalize_text = normalize["result"]["content"][0]["text"].as_str().unwrap();
    assert!(normalize_text.contains("\"kind\": \"normalize\""));
    assert!(normalize_text.contains("\"changed\": true"));
    assert!(normalize_text.contains("> Riley know the deploy before lunch"));

    let split = handle_request(
        json!({"method":"tools/call","id":102,"params":{"name":"mempalace_split","arguments":{
            "source_dir": transcripts,
            "dry_run": "true",
            "min_sessions": "2"
        }}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let split_text = split["result"]["content"][0]["text"].as_str().unwrap();
    assert!(split_text.contains("\"kind\": \"split\""));
    assert!(split_text.contains("\"dry_run\": true"));
    assert!(split_text.contains("\"files_created\": 2"));
}

#[tokio::test]
async fn mcp_helper_tools_work() {
    let tmp = tempdir().unwrap();
    let transcripts = tmp.path().join("session.jsonl");
    let mut lines = Vec::new();
    for idx in 0..15 {
        lines.push(format!(
            "{{\"type\":\"event_msg\",\"payload\":{{\"type\":\"user_message\",\"message\":\"user turn {idx}\"}}}}"
        ));
    }
    std::fs::write(&transcripts, lines.join("\n")).unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    let instructions = handle_request(
        json!({"method":"tools/call","id":130,"params":{"name":"mempalace_instructions","arguments":{"name":"help"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let instructions_text = instructions["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(instructions_text.contains("\"kind\": \"instructions\""));
    assert!(instructions_text.contains("# MemPalace"));
    assert!(instructions_text.contains("Slash Commands"));

    let hook_run = handle_request(
        json!({"method":"tools/call","id":131,"params":{"name":"mempalace_hook_run","arguments":{
            "hook": "stop",
            "harness": "codex",
            "session_id": "demo-session",
            "stop_hook_active": "false",
            "transcript_path": transcripts
        }}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let hook_text = hook_run["result"]["content"][0]["text"].as_str().unwrap();
    assert!(hook_text.contains("\"kind\": \"hook_run\""));
    assert!(hook_text.contains("\"hook\": \"stop\""));
    assert!(hook_text.contains("\"decision\": \"block\""));
    assert!(hook_text.contains("AUTO-SAVE checkpoint"));
}

#[tokio::test]
async fn mcp_project_bootstrap_tools_return_tool_level_errors_for_missing_args() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    for (tool_name, expected_error, expected_hint) in [
        (
            "mempalace_onboarding",
            "Onboarding error: MCP error: mempalace_onboarding requires project_dir",
            "Provide project_dir, then rerun mempalace_onboarding.",
        ),
        (
            "mempalace_normalize",
            "Normalize error: MCP error: mempalace_normalize requires file_path",
            "Provide file_path, then rerun mempalace_normalize.",
        ),
        (
            "mempalace_split",
            "Split error: MCP error: mempalace_split requires source_dir",
            "Provide source_dir, then rerun mempalace_split.",
        ),
    ] {
        let response = handle_request(
            json!({"method":"tools/call","id":110,"params":{"name":tool_name,"arguments":{}}}),
            &config,
        )
        .await
        .unwrap()
        .unwrap();

        assert!(response.get("error").is_none());
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let payload: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(payload["error"].as_str().unwrap(), expected_error);
        assert_eq!(payload["hint"].as_str().unwrap(), expected_hint);
    }
}

#[tokio::test]
async fn mcp_helper_tools_return_tool_level_errors_for_bad_inputs() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    for (tool_name, arguments, expected_error, expected_hint) in [
        (
            "mempalace_instructions",
            json!({}),
            "Instructions error: MCP error: mempalace_instructions requires name",
            "Provide an instruction name, then rerun mempalace_instructions.",
        ),
        (
            "mempalace_hook_run",
            json!({}),
            "Hook run error: MCP error: mempalace_hook_run requires hook",
            "Provide hook and harness, then rerun mempalace_hook_run.",
        ),
        (
            "mempalace_hook_run",
            json!({"hook":"stop","harness":"broken"}),
            "Hook run error: Invalid argument: Unknown harness: broken",
            "Check hook, harness, and transcript_path, then rerun mempalace_hook_run.",
        ),
        (
            "mempalace_instructions",
            json!({"name":"unknown"}),
            "Instructions error: Invalid argument: Unknown instructions: unknown",
            "Use one of help, init, mine, search, or status, then rerun mempalace_instructions.",
        ),
    ] {
        let response = handle_request(
            json!({"method":"tools/call","id":140,"params":{"name":tool_name,"arguments":arguments}}),
            &config,
        )
        .await
        .unwrap()
        .unwrap();

        assert!(response.get("error").is_none());
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let payload: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(payload["error"].as_str().unwrap(), expected_error);
        assert_eq!(payload["hint"].as_str().unwrap(), expected_hint);
    }
}

#[tokio::test]
async fn mcp_project_bootstrap_tools_return_tool_level_errors_for_bad_inputs() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("world");
    std::fs::create_dir_all(&project).unwrap();
    let unsupported = tmp.path().join("unsupported.json");
    std::fs::write(&unsupported, "this is not valid json").unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    let onboarding = handle_request(
        json!({"method":"tools/call","id":120,"params":{"name":"mempalace_onboarding","arguments":{
            "project_dir": project,
            "people": [",,"]
        }}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let onboarding_text = onboarding["result"]["content"][0]["text"].as_str().unwrap();
    let onboarding_payload: serde_json::Value = serde_json::from_str(onboarding_text).unwrap();
    assert_eq!(
        onboarding_payload["error"].as_str().unwrap(),
        "Onboarding error: Invalid argument: Person must include at least a name"
    );
    assert_eq!(
        onboarding_payload["hint"].as_str().unwrap(),
        "Use people entries in name,relationship,context format, then rerun mempalace_onboarding."
    );

    let normalize = handle_request(
        json!({"method":"tools/call","id":121,"params":{"name":"mempalace_normalize","arguments":{
            "file_path": unsupported
        }}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let normalize_text = normalize["result"]["content"][0]["text"].as_str().unwrap();
    let normalize_payload: serde_json::Value = serde_json::from_str(normalize_text).unwrap();
    assert_eq!(
        normalize_payload["error"].as_str().unwrap(),
        "Normalize error: Invalid argument: Unsupported or unreadable conversation file."
    );
    assert_eq!(
        normalize_payload["hint"].as_str().unwrap(),
        "Use a supported .txt, .md, .json, or .jsonl chat export, then rerun mempalace_normalize."
    );
}

#[tokio::test]
async fn mcp_read_tools_return_python_style_no_palace_response() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    let status = handle_request(
        json!({"method":"tools/call","id":1,"params":{"name":"mempalace_status","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    let text = status["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("\"error\": \"No palace found\""));
    assert!(text.contains("Run: mempalace init <dir> && mempalace mine <dir>"));
}

#[tokio::test]
async fn mcp_search_returns_tool_level_error_payload_on_query_failure() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let sqlite_path = config.sqlite_path();
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let sqlite = Connection::open(sqlite_path).unwrap();
    sqlite
        .execute(
            "UPDATE meta SET value = 'broken-provider' WHERE key = 'embedding_provider'",
            [],
        )
        .unwrap();

    let search = handle_request(
        json!({"method":"tools/call","id":4,"params":{"name":"mempalace_search","arguments":{"query":"GraphQL","limit":"3"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    assert!(search.get("error").is_none());
    let search_text = search["result"]["content"][0]["text"].as_str().unwrap();
    assert!(search_text.contains("\"error\": \"Search error:"));
    assert!(search_text.contains("\"hint\":"));
    assert!(search_text.contains("Palace embedding profile mismatch"));
}

#[tokio::test]
async fn mcp_read_tools_return_tool_level_error_payloads_on_broken_sqlite() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let sqlite_path = config.sqlite_path();
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    std::fs::write(&sqlite_path, b"not a sqlite database").unwrap();

    for tool_name in [
        "mempalace_status",
        "mempalace_list_wings",
        "mempalace_list_rooms",
        "mempalace_get_taxonomy",
    ] {
        let response = handle_request(
            json!({"method":"tools/call","id":10,"params":{"name":tool_name,"arguments":{}}}),
            &config,
        )
        .await
        .unwrap()
        .unwrap();

        assert!(
            response.get("error").is_none(),
            "{tool_name} should not raise MCP transport error"
        );
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let payload: serde_json::Value = serde_json::from_str(text).unwrap();
        let error = payload["error"].as_str().unwrap();
        assert!(
            !error.is_empty(),
            "{tool_name} should return tool-level error payload"
        );
        assert!(
            error.contains("malformed")
                || error.contains("not a database")
                || error.contains("disk image is malformed"),
            "{tool_name} should surface SQLite failure, got: {error}"
        );
        assert!(
            payload["hint"].as_str().is_some(),
            "{tool_name} should include recovery hint"
        );
    }
}

#[tokio::test]
async fn mcp_maintenance_tools_return_tool_level_error_payloads_on_broken_sqlite() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let sqlite_path = config.sqlite_path();
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    std::fs::write(&sqlite_path, b"not a sqlite database").unwrap();

    for (tool_name, arguments) in [
        ("mempalace_repair", json!({})),
        ("mempalace_repair_scan", json!({})),
        ("mempalace_repair_rebuild", json!({})),
        ("mempalace_compress", json!({})),
        ("mempalace_dedup", json!({"dry_run": true})),
    ] {
        let response = handle_request(
            json!({"method":"tools/call","id":18,"params":{"name":tool_name,"arguments":arguments}}),
            &config,
        )
        .await
        .unwrap()
        .unwrap();

        assert!(
            response.get("error").is_none(),
            "{tool_name} should not raise MCP transport error"
        );
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let payload: serde_json::Value = serde_json::from_str(text).unwrap();
        let error = payload["error"].as_str().unwrap();
        assert!(
            !error.is_empty(),
            "{tool_name} should return tool-level error payload"
        );
        assert!(
            error.contains("malformed")
                || error.contains("not a database")
                || error.contains("disk image is malformed"),
            "{tool_name} should surface SQLite failure, got: {error}"
        );
        assert!(
            payload["hint"].as_str().is_some(),
            "{tool_name} should include recovery hint"
        );
    }
}

#[tokio::test]
async fn mcp_search_returns_tool_level_error_payload_when_query_is_missing() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let response = handle_request(
        json!({"method":"tools/call","id":11,"params":{"name":"mempalace_search","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    assert!(response.get("error").is_none());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let payload: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        payload["error"].as_str().unwrap(),
        "Search error: MCP error: mempalace_search requires query"
    );
    assert_eq!(
        payload["hint"].as_str().unwrap(),
        "Provide a query string, then rerun mempalace_search."
    );
}

#[tokio::test]
async fn mcp_check_duplicate_returns_tool_level_error_when_content_is_missing() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let response = handle_request(
        json!({"method":"tools/call","id":12,"params":{"name":"mempalace_check_duplicate","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    assert!(response.get("error").is_none());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let payload: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        payload["error"].as_str().unwrap(),
        "Check duplicate error: MCP error: mempalace_check_duplicate requires content"
    );
    assert_eq!(
        payload["hint"].as_str().unwrap(),
        "Provide content text, then rerun mempalace_check_duplicate."
    );
}

#[tokio::test]
async fn mcp_graph_read_tools_work() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    seed_graph_palace(&config).await;

    let traverse = handle_request(
        json!({"method":"tools/call","id":20,"params":{"name":"mempalace_traverse","arguments":{"start_room":"shared-room","max_hops":"2"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let traverse_text = traverse["result"]["content"][0]["text"].as_str().unwrap();
    assert!(traverse_text.contains("\"room\": \"shared-room\""));
    assert!(traverse_text.contains("\"hop\": 0"));
    assert!(traverse_text.contains("\"wing_code\""));
    assert!(traverse_text.contains("\"wing_team\""));

    let tunnels = handle_request(
        json!({"method":"tools/call","id":21,"params":{"name":"mempalace_find_tunnels","arguments":{"wing_a":"wing_code","wing_b":"wing_team"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let tunnels_text = tunnels["result"]["content"][0]["text"].as_str().unwrap();
    assert!(tunnels_text.contains("\"room\": \"shared-room\""));
    assert!(tunnels_text.contains("\"recent\""));

    let stats = handle_request(
        json!({"method":"tools/call","id":22,"params":{"name":"mempalace_graph_stats","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let stats_text = stats["result"]["content"][0]["text"].as_str().unwrap();
    assert!(stats_text.contains("\"total_rooms\": 2"));
    assert!(stats_text.contains("\"tunnel_rooms\": 1"));
    assert!(stats_text.contains("\"total_edges\": 1"));
    assert!(stats_text.contains("\"top_tunnels\""));
}

#[tokio::test]
async fn mcp_traverse_returns_python_style_not_found_payload() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    seed_graph_palace(&config).await;

    let response = handle_request(
        json!({"method":"tools/call","id":23,"params":{"name":"mempalace_traverse","arguments":{"start_room":"shared"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("\"error\": \"Room 'shared' not found\""));
    assert!(text.contains("\"suggestions\""));
    assert!(text.contains("shared-room"));
}

#[tokio::test]
async fn mcp_traverse_returns_tool_level_error_when_start_room_is_missing() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let response = handle_request(
        json!({"method":"tools/call","id":24,"params":{"name":"mempalace_traverse","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let payload: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        payload["error"].as_str().unwrap(),
        "Traverse error: MCP error: mempalace_traverse requires start_room"
    );
    assert_eq!(
        payload["hint"].as_str().unwrap(),
        "Provide a start_room value, then rerun mempalace_traverse."
    );
}

#[tokio::test]
async fn mcp_kg_read_tools_work() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    seed_kg_palace(&config).await;

    let query = handle_request(
        json!({"method":"tools/call","id":30,"params":{"name":"mempalace_kg_query","arguments":{"entity":"Max","direction":"both","as_of":"2026-01-15"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let query_text = query["result"]["content"][0]["text"].as_str().unwrap();
    assert!(query_text.contains("\"entity\": \"Max\""));
    assert!(query_text.contains("\"count\": 3"));
    assert!(query_text.contains("\"predicate\": \"loves\""));
    assert!(query_text.contains("\"predicate\": \"child_of\""));
    assert!(query_text.contains("\"predicate\": \"has_issue\""));

    let timeline = handle_request(
        json!({"method":"tools/call","id":31,"params":{"name":"mempalace_kg_timeline","arguments":{"entity":"Max"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let timeline_text = timeline["result"]["content"][0]["text"].as_str().unwrap();
    assert!(timeline_text.contains("\"entity\": \"Max\""));
    assert!(timeline_text.contains("\"timeline\""));
    assert!(timeline_text.contains("\"current\": true"));

    let stats = handle_request(
        json!({"method":"tools/call","id":32,"params":{"name":"mempalace_kg_stats","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let stats_text = stats["result"]["content"][0]["text"].as_str().unwrap();
    assert!(stats_text.contains("\"entities\": 4"));
    assert!(stats_text.contains("\"triples\": 3"));
    assert!(stats_text.contains("\"current_facts\": 2"));
    assert!(stats_text.contains("\"expired_facts\": 1"));
    assert!(stats_text.contains("\"relationship_types\""));
}

#[tokio::test]
async fn mcp_kg_query_returns_tool_level_error_for_bad_direction() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let response = handle_request(
        json!({"method":"tools/call","id":33,"params":{"name":"mempalace_kg_query","arguments":{"entity":"Max","direction":"sideways"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let payload: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        payload["error"].as_str().unwrap(),
        "KG query error: MCP error: unsupported direction: sideways"
    );
    assert_eq!(
        payload["hint"].as_str().unwrap(),
        "Use direction=outgoing, incoming, or both, then rerun mempalace_kg_query."
    );
}

#[tokio::test]
async fn mcp_kg_write_tools_work() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    let add = handle_request(
        json!({"method":"tools/call","id":34,"params":{"name":"mempalace_kg_add","arguments":{"subject":"Max","predicate":"works_on","object":"Mempalace","valid_from":"2026-04-14"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let add_text = add["result"]["content"][0]["text"].as_str().unwrap();
    assert!(add_text.contains("\"success\": true"));
    assert!(add_text.contains("\"triple_id\""));
    assert!(add_text.contains("Max → works_on → Mempalace"));

    let invalidate = handle_request(
        json!({"method":"tools/call","id":35,"params":{"name":"mempalace_kg_invalidate","arguments":{"subject":"Max","predicate":"works_on","object":"Mempalace","ended":"2026-04-15"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let invalidate_text = invalidate["result"]["content"][0]["text"].as_str().unwrap();
    assert!(invalidate_text.contains("\"success\": true"));
    assert!(invalidate_text.contains("\"updated\": 1"));
    assert!(invalidate_text.contains("\"ended\": \"2026-04-15\""));
}

#[tokio::test]
async fn mcp_kg_write_tools_return_tool_level_errors_for_missing_args() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let add = handle_request(
        json!({"method":"tools/call","id":36,"params":{"name":"mempalace_kg_add","arguments":{"predicate":"works_on","object":"Mempalace"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let add_payload: serde_json::Value =
        serde_json::from_str(add["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        add_payload["error"].as_str().unwrap(),
        "KG add error: MCP error: mempalace_kg_add requires subject"
    );

    let invalidate = handle_request(
        json!({"method":"tools/call","id":37,"params":{"name":"mempalace_kg_invalidate","arguments":{"subject":"Max","object":"Mempalace"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let invalidate_payload: serde_json::Value =
        serde_json::from_str(invalidate["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        invalidate_payload["error"].as_str().unwrap(),
        "KG invalidate error: MCP error: mempalace_kg_invalidate requires predicate"
    );
}

#[tokio::test]
async fn mcp_add_and_delete_drawer_work() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    let add = handle_request(
        json!({"method":"tools/call","id":38,"params":{"name":"mempalace_add_drawer","arguments":{"wing":"Project Notes","room":"Backend","content":"Verbatim architecture notes for the Rust rewrite.","source_file":"notes/backend.md","added_by":"codex"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let add_payload: serde_json::Value =
        serde_json::from_str(add["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(add_payload["success"], true);
    let drawer_id = add_payload["drawer_id"].as_str().unwrap().to_string();
    assert_eq!(add_payload["wing"], "Project Notes");
    assert_eq!(add_payload["room"], "Backend");

    let sqlite = SqliteStore::open(&config.sqlite_path()).unwrap();
    assert_eq!(sqlite.total_drawers().unwrap(), 1);

    let delete = handle_request(
        json!({"method":"tools/call","id":39,"params":{"name":"mempalace_delete_drawer","arguments":{"drawer_id":drawer_id}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let delete_payload: serde_json::Value =
        serde_json::from_str(delete["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(delete_payload["success"], true);

    let sqlite = SqliteStore::open(&config.sqlite_path()).unwrap();
    assert_eq!(sqlite.total_drawers().unwrap(), 0);
}

#[tokio::test]
async fn mcp_drawer_write_tools_return_tool_level_errors_for_missing_args() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let add = handle_request(
        json!({"method":"tools/call","id":40,"params":{"name":"mempalace_add_drawer","arguments":{"room":"backend","content":"hello"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let add_payload: serde_json::Value =
        serde_json::from_str(add["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        add_payload["error"].as_str().unwrap(),
        "Add drawer error: MCP error: mempalace_add_drawer requires wing"
    );

    let delete = handle_request(
        json!({"method":"tools/call","id":41,"params":{"name":"mempalace_delete_drawer","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let delete_payload: serde_json::Value =
        serde_json::from_str(delete["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        delete_payload["error"].as_str().unwrap(),
        "Delete drawer error: MCP error: mempalace_delete_drawer requires drawer_id"
    );
}

#[tokio::test]
async fn mcp_diary_write_and_read_work() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    let write = handle_request(
        json!({"method":"tools/call","id":40,"params":{"name":"mempalace_diary_write","arguments":{"agent_name":"Codex","entry":"SESSION: shipped KG read tools","topic":"release"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let write_text = write["result"]["content"][0]["text"].as_str().unwrap();
    assert!(write_text.contains("\"success\": true"));
    assert!(write_text.contains("\"agent\": \"Codex\""));
    assert!(write_text.contains("\"topic\": \"release\""));

    let read = handle_request(
        json!({"method":"tools/call","id":41,"params":{"name":"mempalace_diary_read","arguments":{"agent_name":"Codex","last_n":"5"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let read_text = read["result"]["content"][0]["text"].as_str().unwrap();
    assert!(read_text.contains("\"agent\": \"Codex\""));
    assert!(read_text.contains("\"entries\""));
    assert!(read_text.contains("SESSION: shipped KG read tools"));
    assert!(read_text.contains("\"showing\": 1"));
}

#[tokio::test]
async fn mcp_registry_tools_work() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("notes.md"),
        "Ever said Atlas should launch soon.\nEver wrote the Atlas architecture notes.\nEver pushed the Atlas repo.\nhey Ever, should Atlas ship?\n",
    )
    .unwrap();

    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init_project(&project).await.unwrap();

    let summary = handle_request(
        json!({"method":"tools/call","id":60,"params":{"name":"mempalace_registry_summary","arguments":{"project_dir":project}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let summary_text = summary["result"]["content"][0]["text"].as_str().unwrap();
    assert!(summary_text.contains("\"kind\": \"registry_summary\""));
    assert!(summary_text.contains("\"people_count\""));

    let lookup = handle_request(
        json!({"method":"tools/call","id":61,"params":{"name":"mempalace_registry_lookup","arguments":{"project_dir":project,"word":"Ever","context":"Have you ever seen this before?"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let lookup_text = lookup["result"]["content"][0]["text"].as_str().unwrap();
    assert!(lookup_text.contains("\"type\": \"concept\""));

    let add_person = handle_request(
        json!({"method":"tools/call","id":62,"params":{"name":"mempalace_registry_add_person","arguments":{"project_dir":project,"name":"Riley","relationship":"daughter","context":"personal"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let add_person_text = add_person["result"]["content"][0]["text"].as_str().unwrap();
    assert!(add_person_text.contains("\"action\": \"add_person\""));

    let add_project = handle_request(
        json!({"method":"tools/call","id":63,"params":{"name":"mempalace_registry_add_project","arguments":{"project_dir":project,"name":"Lantern"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let add_project_text = add_project["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(add_project_text.contains("\"action\": \"add_project\""));

    let add_alias = handle_request(
        json!({"method":"tools/call","id":64,"params":{"name":"mempalace_registry_add_alias","arguments":{"project_dir":project,"canonical":"Riley","alias":"Ry"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let add_alias_text = add_alias["result"]["content"][0]["text"].as_str().unwrap();
    assert!(add_alias_text.contains("\"action\": \"add_alias\""));

    let query = handle_request(
        json!({"method":"tools/call","id":65,"params":{"name":"mempalace_registry_query","arguments":{"project_dir":project,"query":"Ry said Lantern should ship with Max"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let query_text = query["result"]["content"][0]["text"].as_str().unwrap();
    assert!(query_text.contains("\"people\""));
    assert!(query_text.contains("Riley"));
    assert!(query_text.contains("Max"));

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

    let research = handle_request(
        json!({"method":"tools/call","id":66,"params":{"name":"mempalace_registry_research","arguments":{"project_dir":project,"word":"Max"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let research_text = research["result"]["content"][0]["text"].as_str().unwrap();
    assert!(research_text.contains("\"kind\": \"registry_research\""));
    assert!(research_text.contains("\"word\": \"Max\""));

    let confirm = handle_request(
        json!({"method":"tools/call","id":67,"params":{"name":"mempalace_registry_confirm","arguments":{"project_dir":project,"word":"Max","entity_type":"person","relationship":"coworker","context":"work"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let confirm_text = confirm["result"]["content"][0]["text"].as_str().unwrap();
    assert!(confirm_text.contains("\"kind\": \"registry_confirm\""));
    assert!(confirm_text.contains("\"entity_type\": \"person\""));
}

#[tokio::test]
async fn mcp_registry_tools_return_tool_level_errors_for_missing_args() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let response = handle_request(
        json!({"method":"tools/call","id":68,"params":{"name":"mempalace_registry_lookup","arguments":{"project_dir":"."}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let payload: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        payload["error"].as_str().unwrap(),
        "Registry lookup error: MCP error: mempalace_registry_lookup requires word"
    );

    let response = handle_request(
        json!({"method":"tools/call","id":69,"params":{"name":"mempalace_registry_confirm","arguments":{"project_dir":"."}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let payload: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        payload["error"].as_str().unwrap(),
        "Registry confirm error: MCP error: mempalace_registry_confirm requires word"
    );
}

#[tokio::test]
async fn mcp_diary_read_returns_empty_message_for_new_agent() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let response = handle_request(
        json!({"method":"tools/call","id":42,"params":{"name":"mempalace_diary_read","arguments":{"agent_name":"Codex"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("\"message\": \"No diary entries yet.\""));
    assert!(text.contains("\"entries\": []"));
}

#[tokio::test]
async fn mcp_diary_tools_return_tool_level_errors_for_missing_args() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let write = handle_request(
        json!({"method":"tools/call","id":43,"params":{"name":"mempalace_diary_write","arguments":{"agent_name":"Codex"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let write_payload: serde_json::Value =
        serde_json::from_str(write["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        write_payload["error"].as_str().unwrap(),
        "Diary write error: MCP error: mempalace_diary_write requires entry"
    );

    let read = handle_request(
        json!({"method":"tools/call","id":44,"params":{"name":"mempalace_diary_read","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    let read_payload: serde_json::Value =
        serde_json::from_str(read["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        read_payload["error"].as_str().unwrap(),
        "Diary read error: MCP error: mempalace_diary_read requires agent_name"
    );
}

#[tokio::test]
async fn mcp_write_tools_append_palace_local_wal_entries() {
    let tmp = tempdir().unwrap();
    let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    config.embedding.backend = EmbeddingBackend::Hash;

    handle_request(
        json!({"method":"tools/call","id":50,"params":{"name":"mempalace_add_drawer","arguments":{"wing":"Project Notes","room":"Backend","content":"Verbatim architecture notes for WAL coverage.","added_by":"codex"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    handle_request(
        json!({"method":"tools/call","id":51,"params":{"name":"mempalace_kg_add","arguments":{"subject":"Max","predicate":"works_on","object":"WAL","valid_from":"2026-04-14"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    handle_request(
        json!({"method":"tools/call","id":52,"params":{"name":"mempalace_diary_write","arguments":{"agent_name":"Codex","entry":"SESSION: tested WAL logging","topic":"audit"}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();

    let wal_path = config.palace_path.join("wal").join("write_log.jsonl");
    assert!(wal_path.exists());

    let lines = std::fs::read_to_string(&wal_path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["operation"], "add_drawer");
    assert_eq!(lines[1]["operation"], "kg_add");
    assert_eq!(lines[2]["operation"], "diary_write");
    assert_eq!(lines[0]["params"]["added_by"], "codex");
    assert_eq!(lines[1]["params"]["subject"], "Max");
    assert_eq!(lines[2]["params"]["topic"], "audit");
    assert!(
        lines[0]["params"]["content_preview"]
            .as_str()
            .unwrap()
            .contains("Verbatim")
    );
}

async fn seed_graph_palace(config: &AppConfig) {
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();

    let mut sqlite = SqliteStore::open(&config.sqlite_path()).unwrap();
    sqlite.init_schema().unwrap();
    sqlite
        .replace_source(
            "graph://code/shared",
            "wing_code",
            "shared-room",
            "hash-a",
            Some(1.0),
            &[DrawerInput {
                id: "drawer-code-shared".to_string(),
                wing: "wing_code".to_string(),
                room: "shared-room".to_string(),
                source_file: "shared-code.md".to_string(),
                source_path: "graph://code/shared".to_string(),
                source_hash: "hash-a".to_string(),
                source_mtime: Some(1.0),
                chunk_index: 0,
                added_by: "test".to_string(),
                filed_at: "2026-04-14T10:00:00Z".to_string(),
                ingest_mode: "projects".to_string(),
                extract_mode: "exchange".to_string(),
                text: "shared room in code wing".to_string(),
            }],
        )
        .unwrap();
    sqlite
        .replace_source(
            "graph://team/shared",
            "wing_team",
            "shared-room",
            "hash-b",
            Some(2.0),
            &[DrawerInput {
                id: "drawer-team-shared".to_string(),
                wing: "wing_team".to_string(),
                room: "shared-room".to_string(),
                source_file: "shared-team.md".to_string(),
                source_path: "graph://team/shared".to_string(),
                source_hash: "hash-b".to_string(),
                source_mtime: Some(2.0),
                chunk_index: 0,
                added_by: "test".to_string(),
                filed_at: "2026-04-14T11:00:00Z".to_string(),
                ingest_mode: "projects".to_string(),
                extract_mode: "exchange".to_string(),
                text: "shared room in team wing".to_string(),
            }],
        )
        .unwrap();
    sqlite
        .replace_source(
            "graph://team/solo",
            "wing_team",
            "solo-room",
            "hash-c",
            Some(3.0),
            &[DrawerInput {
                id: "drawer-team-solo".to_string(),
                wing: "wing_team".to_string(),
                room: "solo-room".to_string(),
                source_file: "solo-team.md".to_string(),
                source_path: "graph://team/solo".to_string(),
                source_hash: "hash-c".to_string(),
                source_mtime: Some(3.0),
                chunk_index: 0,
                added_by: "test".to_string(),
                filed_at: "2026-04-14T12:00:00Z".to_string(),
                ingest_mode: "projects".to_string(),
                extract_mode: "exchange".to_string(),
                text: "solo room in team wing".to_string(),
            }],
        )
        .unwrap();
}

async fn seed_kg_palace(config: &AppConfig) {
    let app = App::new(config.clone()).unwrap();
    app.init().await.unwrap();
    app.add_kg_triple(&KgTriple {
        subject: "Max".to_string(),
        predicate: "child_of".to_string(),
        object: "Alice".to_string(),
        valid_from: Some("2015-04-01".to_string()),
        valid_to: None,
    })
    .await
    .unwrap();
    app.add_kg_triple(&KgTriple {
        subject: "Max".to_string(),
        predicate: "loves".to_string(),
        object: "Chess".to_string(),
        valid_from: Some("2025-10-01".to_string()),
        valid_to: None,
    })
    .await
    .unwrap();
    app.add_kg_triple(&KgTriple {
        subject: "Max".to_string(),
        predicate: "has_issue".to_string(),
        object: "Sports injury".to_string(),
        valid_from: Some("2026-01-01".to_string()),
        valid_to: Some("2026-02-15".to_string()),
    })
    .await
    .unwrap();
}
