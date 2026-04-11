use mempalace_rs::config::AppConfig;
use mempalace_rs::model::KgTriple;
use mempalace_rs::service::App;
use tempfile::tempdir;

#[tokio::test]
async fn init_is_idempotent_and_status_starts_empty() {
    let tmp = tempdir().unwrap();
    let config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    let app = App::new(config);

    let first = app.init().await.unwrap();
    let second = app.init().await.unwrap();
    let status = app.status().await.unwrap();

    assert_eq!(first.palace_path, second.palace_path);
    assert_eq!(status.total_drawers, 0);
    assert!(status.wings.is_empty());
    assert!(status.rooms.is_empty());
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

    let config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
    let app = App::new(config);
    app.init().await.unwrap();
    app.mine_project(&project, Some("project"), 0, true, &[])
        .await
        .unwrap();

    let taxonomy = app.taxonomy().await.unwrap();
    assert_eq!(taxonomy.taxonomy["project"]["src"], 1);

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
