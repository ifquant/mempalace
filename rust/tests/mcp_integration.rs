use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::mcp::handle_request;
use mempalace_rs::service::App;
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
    app.mine_project(&project, Some("project"), 0, false, true, &[])
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
