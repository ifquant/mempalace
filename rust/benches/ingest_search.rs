use std::fs;

use criterion::{Criterion, criterion_group, criterion_main};
use mempalace_rs::config::{AppConfig, EmbeddingBackend};
use mempalace_rs::service::App;
use tempfile::tempdir;

fn ingest_search_benchmark(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    criterion.bench_function("mine_project_small_fixture", |bench| {
        bench.iter(|| {
            runtime.block_on(async {
                let tmp = tempdir().unwrap();
                let project = tmp.path().join("project");
                fs::create_dir_all(project.join("src")).unwrap();
                fs::write(
                    project.join("src").join("notes.txt"),
                    "Planning notes for Rust rewrite.\n\nLanceDB stores vectors locally.",
                )
                .unwrap();

                let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
                config.embedding.backend = EmbeddingBackend::Hash;
                let app = App::new(config).unwrap();
                app.init().await.unwrap();
                app.mine_project(&project, Some("project"), 0, false, true, &[])
                    .await
                    .unwrap();
            });
        });
    });

    criterion.bench_function("search_small_fixture", |bench| {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("src")).unwrap();
        fs::write(
            project.join("src").join("notes.txt"),
            "Planning notes for Rust rewrite.\n\nLanceDB stores vectors locally.",
        )
        .unwrap();

        let mut config = AppConfig::resolve(Some(tmp.path().join("palace"))).unwrap();
        config.embedding.backend = EmbeddingBackend::Hash;
        let app = App::new(config).unwrap();
        runtime.block_on(async {
            app.init().await.unwrap();
            app.mine_project(&project, Some("project"), 0, false, true, &[])
                .await
                .unwrap();
        });

        bench.iter(|| {
            runtime.block_on(async {
                let results = app
                    .search("vectors", Some("project"), None, 3)
                    .await
                    .unwrap();
                assert!(!results.results.is_empty());
            });
        });
    });
}

criterion_group!(benches, ingest_search_benchmark);
criterion_main!(benches);
