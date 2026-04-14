use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::mcp::handle_request;
use mempalace_rs::model::MineRequest;
use mempalace_rs::service::App;
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
