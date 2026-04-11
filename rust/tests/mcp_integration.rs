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
    app.mine_project(&project, Some("project"), 0, true, &[])
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

    let status = handle_request(
        json!({"method":"tools/call","id":3,"params":{"name":"mempalace_status","arguments":{}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        status["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("total_drawers")
    );

    let search = handle_request(
        json!({"method":"tools/call","id":4,"params":{"name":"mempalace_search","arguments":{"query":"GraphQL","limit":3}}}),
        &config,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        search["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("GraphQL")
    );
}
