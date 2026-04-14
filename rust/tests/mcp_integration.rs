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
    assert!(tool_names.contains(&"mempalace_kg_query"));
    assert!(tool_names.contains(&"mempalace_kg_timeline"));
    assert!(tool_names.contains(&"mempalace_kg_stats"));
    assert!(tool_names.contains(&"mempalace_traverse"));
    assert!(tool_names.contains(&"mempalace_find_tunnels"));
    assert!(tool_names.contains(&"mempalace_graph_stats"));
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
