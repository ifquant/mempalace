use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use mempalace_rs::config::AppConfig;
use mempalace_rs::mcp;
use mempalace_rs::model::{MineProgressEvent, MineRequest};
use mempalace_rs::service::App;
use serde_json::json;

#[derive(Parser)]
#[command(name = "mempalace-rs")]
#[command(
    about = "MemPalace — Give your AI a memory. No API key required.",
    long_about = "MemPalace — Give your AI a memory. No API key required.\n\nCurrent Rust phase supports local-first project mining, search, migration, repair diagnostics, and read-only MCP tools.\n\nExamples:\n  mempalace-rs init ~/projects/my_app\n  mempalace-rs mine ~/projects/my_app\n  mempalace-rs search \"why did we switch to GraphQL\"\n  mempalace-rs status\n  mempalace-rs migrate\n  mempalace-rs repair"
)]
struct Cli {
    #[arg(long)]
    #[arg(
        help = "Where the palace lives (default: ~/.mempalace-rs/palace or MEMPALACE_RS_PALACE_PATH)"
    )]
    palace: Option<PathBuf>,
    #[arg(long)]
    #[arg(help = "Override the HuggingFace endpoint used by fastembed model downloads")]
    hf_endpoint: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Set up a palace directory for a project")]
    Init {
        #[arg(help = "Project directory to set up")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable init summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Mine project files into the palace")]
    Mine {
        #[arg(help = "Directory to mine")]
        dir: PathBuf,
        #[arg(long, default_value = "projects")]
        #[arg(help = "Ingest mode: 'projects' for code/docs (default), 'convos' for chat exports")]
        mode: String,
        #[arg(long)]
        #[arg(help = "Wing name (default: mempalace.yaml wing or directory name)")]
        wing: Option<String>,
        #[arg(long, default_value_t = 0)]
        #[arg(help = "Max files to process (0 = all)")]
        limit: usize,
        #[arg(long)]
        #[arg(help = "Preview what would be mined without writing drawers to the palace")]
        dry_run: bool,
        #[arg(long)]
        #[arg(help = "Do not respect .gitignore files when scanning project files")]
        no_gitignore: bool,
        #[arg(long = "include-ignored")]
        #[arg(
            help = "Always scan these project-relative paths even if ignored; repeat or pass comma-separated paths"
        )]
        include_ignored: Vec<String>,
        #[arg(long, default_value = "mempalace")]
        #[arg(help = "Your name — recorded on every drawer (default: mempalace)")]
        agent: String,
        #[arg(long, default_value = "exchange")]
        #[arg(
            help = "Extraction strategy for convos mode: 'exchange' (default) or 'general' (5 memory types)"
        )]
        extract: String,
        #[arg(long)]
        #[arg(
            help = "Print Python-style per-file mining progress to stderr while keeping JSON on stdout"
        )]
        progress: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable mine summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Find anything, exact words")]
    Search {
        #[arg(help = "What to search for")]
        query: String,
        #[arg(long)]
        #[arg(help = "Limit to one project/wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Limit to one room")]
        room: Option<String>,
        #[arg(long, default_value_t = 5)]
        #[arg(help = "Number of results")]
        results: usize,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable search output instead of JSON")]
        human: bool,
    },
    #[command(about = "Upgrade palace SQLite metadata to the current schema version")]
    Migrate {
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable migration summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Run non-destructive palace diagnostics")]
    Repair {
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable repair diagnostics instead of JSON")]
        human: bool,
    },
    #[command(about = "Show what has been filed in the palace")]
    Status {
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable palace status instead of JSON")]
        human: bool,
    },
    #[command(about = "Inspect embedding runtime health and cache state")]
    Doctor {
        #[arg(long)]
        #[arg(help = "Warm the embedding model during the doctor run")]
        warm_embedding: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable doctor output instead of JSON")]
        human: bool,
    },
    #[command(about = "Prepare the local embedding runtime and model cache")]
    PrepareEmbedding {
        #[arg(long, default_value_t = 3)]
        #[arg(help = "How many warm-up attempts to make")]
        attempts: usize,
        #[arg(long, default_value_t = 1000)]
        #[arg(help = "Milliseconds to wait between attempts")]
        wait_ms: u64,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable prepare summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Run the read-only MCP server on stdio")]
    Mcp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Cli {
        palace,
        hf_endpoint,
        command,
    } = Cli::parse();

    match command {
        Command::Init { dir, human } => {
            let palace_path = palace.as_ref().unwrap_or(&dir);
            let mut config = AppConfig::resolve(Some(palace_path))?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
            let summary = app.init().await?;
            if human {
                print_init_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Mine {
            dir,
            mode,
            wing,
            limit,
            dry_run,
            no_gitignore,
            include_ignored,
            agent,
            extract,
            progress,
            human,
        } => {
            if mode != "projects" {
                print_unsupported_mine_mode(&mode, &extract, &dir, human)?;
                std::process::exit(2);
            }
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
            let request = MineRequest {
                wing,
                mode,
                agent,
                limit,
                dry_run,
                respect_gitignore: !no_gitignore,
                include_ignored,
                extract,
            };
            let summary = if progress {
                app.mine_project_with_progress(&dir, &request, |event| match event {
                    MineProgressEvent::DryRun {
                        file_name,
                        room,
                        drawers,
                    } => {
                        eprintln!("    [DRY RUN] {file_name} -> room:{room} ({drawers} drawers)");
                    }
                    MineProgressEvent::Filed {
                        index,
                        total,
                        file_name,
                        drawers,
                    } => {
                        eprintln!("  [ {index:>4}/{total}] {file_name:<50} +{drawers}");
                    }
                })
                .await?
            } else {
                app.mine_project(&dir, &request).await?
            };
            if human {
                print_mine_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Search {
            query,
            wing,
            room,
            results,
            human,
        } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if !palace_exists(&config) {
                if human {
                    print_search_no_palace_human(&config);
                } else {
                    print_no_palace(&config)?;
                }
                std::process::exit(1);
            }
            let app = App::new(config)?;
            let summary = match app
                .search(&query, wing.as_deref(), room.as_deref(), results)
                .await
            {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_search_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => {
                    print_search_error_json(&err.to_string())?;
                    std::process::exit(1);
                }
            };
            if human {
                print_search_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Migrate { human } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if human && !palace_exists(&config) {
                print_migrate_no_palace_human(&config);
                return Ok(());
            }
            let app = App::new(config)?;
            let summary = match app.migrate().await {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_migrate_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => return Err(err.into()),
            };
            if human {
                print_migrate_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Repair { human } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if human && !palace_exists(&config) {
                print_repair_no_palace_human(&config);
                return Ok(());
            }
            let app = App::new(config)?;
            let summary = match app.repair().await {
                Ok(summary) => summary,
                Err(err) if human => {
                    print_repair_error_human(&err.to_string());
                    std::process::exit(1);
                }
                Err(err) => return Err(err.into()),
            };
            if human {
                print_repair_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Status { human } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            if !palace_exists(&config) {
                if human {
                    print_status_no_palace_human(&config);
                } else {
                    print_no_palace(&config)?;
                }
                return Ok(());
            }
            let app = App::new(config)?;
            let summary = app.status().await?;
            if human {
                let taxonomy = app.taxonomy().await?;
                print_status_human(&summary, &taxonomy);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Doctor {
            warm_embedding,
            human,
        } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
            let summary = app.doctor(warm_embedding).await?;
            if human {
                print_doctor_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::PrepareEmbedding {
            attempts,
            wait_ms,
            human,
        } => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            let app = App::new(config)?;
            let summary = app.prepare_embedding(attempts, wait_ms).await?;
            if human {
                print_prepare_embedding_human(&summary);
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
        }
        Command::Mcp => {
            let mut config = AppConfig::resolve(palace.as_ref())?;
            apply_cli_overrides(&mut config, hf_endpoint.as_deref());
            mcp::run_stdio(config).await?;
        }
    }

    Ok(())
}

fn apply_cli_overrides(config: &mut AppConfig, hf_endpoint: Option<&str>) {
    if let Some(endpoint) = hf_endpoint {
        config.embedding.hf_endpoint = Some(endpoint.to_string());
    }
}

fn palace_exists(config: &AppConfig) -> bool {
    config.sqlite_path().exists() || config.lance_path().exists()
}

fn print_no_palace(config: &AppConfig) -> anyhow::Result<()> {
    let payload = json!({
        "error": "No palace found",
        "hint": "Run: mempalace init <dir> && mempalace mine <dir>",
        "palace_path": config.palace_path.display().to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_search_human(summary: &mempalace_rs::model::SearchResults) {
    if summary.results.is_empty() {
        println!("\n  No results found for: \"{}\"", summary.query);
        return;
    }

    println!("\n{}", "=".repeat(60));
    println!("  Results for: \"{}\"", summary.query);
    if let Some(wing) = &summary.filters.wing {
        println!("  Wing: {wing}");
    }
    if let Some(room) = &summary.filters.room {
        println!("  Room: {room}");
    }
    println!("{}\n", "=".repeat(60));

    for (index, hit) in summary.results.iter().enumerate() {
        let similarity = hit
            .similarity
            .map(|value| value.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!("  [{}] {} / {}", index + 1, hit.wing, hit.room);
        println!("      Source: {}", hit.source_file);
        println!("      Match:  {similarity}");
        println!();
        for line in hit.text.trim().lines() {
            println!("      {line}");
        }
        println!();
        println!("  {}", "─".repeat(56));
    }
    println!();
}

fn print_search_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
    println!("  Run: mempalace init <dir> then mempalace mine <dir>");
}

fn print_search_error_human(message: &str) {
    println!("\n  Search error: {message}");
}

fn print_search_error_json(message: &str) -> anyhow::Result<()> {
    let payload = json!({
        "error": format!("Search error: {message}"),
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_status_human(
    summary: &mempalace_rs::model::Status,
    taxonomy: &mempalace_rs::model::Taxonomy,
) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Status — {} drawers", summary.total_drawers);
    println!("{}\n", "=".repeat(55));
    if summary.total_drawers == 0 {
        println!("  Palace is initialized but still empty.");
        println!("  Run: mempalace mine <dir>");
        println!();
    }
    for (wing, rooms) in &taxonomy.taxonomy {
        println!("  WING: {wing}");
        let mut room_entries = rooms.iter().collect::<Vec<_>>();
        room_entries.sort_by(|(left_room, left_count), (right_room, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| left_room.cmp(right_room))
        });
        for (room, count) in room_entries {
            println!("    ROOM: {room:20} {count:5} drawers");
        }
        println!();
    }
    println!("{}", "=".repeat(55));
    println!();
}

fn print_status_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
    println!("  Run: mempalace init <dir> then mempalace mine <dir>");
}

fn print_repair_human(summary: &mempalace_rs::model::RepairSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Repair");
    println!("{}\n", "=".repeat(55));
    println!("  Palace: {}", summary.palace_path);
    println!(
        "  SQLite: {}",
        if summary.sqlite_exists {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "  LanceDB: {}",
        if summary.lance_exists {
            "present"
        } else {
            "missing"
        }
    );
    if let Some(drawers) = summary.sqlite_drawer_count {
        println!("  Drawers found: {drawers}");
    }
    if let Some(version) = summary.schema_version {
        println!("  Schema version: {version}");
    }
    if let Some(provider) = &summary.embedding_provider {
        let model = summary.embedding_model.as_deref().unwrap_or("unknown");
        let dimension = summary
            .embedding_dimension
            .map(|value| value.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!("  Embedding: {provider}/{model}/{dimension}");
    }
    println!(
        "  Vector access: {}",
        if summary.vector_accessible {
            "ok"
        } else {
            "failed"
        }
    );

    if summary.issues.is_empty() {
        println!("\n  Repair diagnostics look healthy.");
    } else {
        println!("\n  Repair diagnostics found problems.");
        println!("\n  Issues:");
        for issue in &summary.issues {
            println!("    - {issue}");
        }
        println!("\n  Suggested next step:");
        println!(
            "    Fix the missing or mismatched palace components, then rerun `mempalace-rs repair`."
        );
    }

    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_repair_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
}

fn print_repair_error_human(message: &str) {
    println!("\n  Repair error: {message}");
    println!("  Check the palace files, then rerun `mempalace-rs repair`.");
}

fn print_migrate_human(summary: &mempalace_rs::model::MigrateSummary) {
    println!("\n{}", "=".repeat(60));
    println!("  MemPalace Migrate");
    println!("{}\n", "=".repeat(60));
    println!("  Palace:  {}", summary.palace_path);
    println!("  SQLite:  {}", summary.sqlite_path);
    println!(
        "  Before:  {}",
        summary
            .schema_version_before
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!("  After:   {}", summary.schema_version_after);
    if summary.changed {
        println!("\n  Migration complete.");
    } else {
        println!("\n  Nothing to migrate.");
    }
    println!("\n{}", "=".repeat(60));
    println!();
}

fn print_migrate_no_palace_human(config: &AppConfig) {
    println!("\n  No palace found at {}", config.palace_path.display());
}

fn print_migrate_error_human(message: &str) {
    println!("\n  Migrate error: {message}");
    println!("  Check the palace SQLite file, then rerun `mempalace-rs migrate`.");
}

fn print_init_human(summary: &mempalace_rs::model::InitSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Init");
    println!("{}\n", "=".repeat(55));
    println!("  Palace:  {}", summary.palace_path);
    println!("  SQLite:  {}", summary.sqlite_path);
    println!("  LanceDB: {}", summary.lance_path);
    println!("  Schema:  {}", summary.schema_version);
    println!("\n  Palace initialized.");
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_mine_human(summary: &mempalace_rs::model::MineSummary) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Mine");
    println!("{}\n", "=".repeat(55));
    println!("  Wing:     {}", summary.wing);
    println!("  Rooms:    {}", summary.configured_rooms.join(", "));
    println!("  Files:    {}", summary.files_planned);
    println!("  Palace:   {}", summary.palace_path);
    println!("  Project:  {}", summary.project_path);
    println!();
    println!("  Files processed: {}", summary.files_mined);
    println!("  Files skipped:   {}", summary.files_skipped_unchanged);
    if summary.dry_run {
        println!("  Drawers previewed: {}", summary.drawers_added);
        println!("  Mode:            DRY RUN");
        println!("  Persistence:     preview only, no drawers were written");
    } else {
        println!("  Drawers filed:   {}", summary.drawers_added);
    }
    if summary.files_planned == 0 {
        println!("\n  No matching files found.");
        println!("  Check your project path, ignore rules, and supported file types.");
    }
    if !summary.room_counts.is_empty() {
        println!("\n  Rooms filed:");
        for (room, count) in &summary.room_counts {
            println!("    - {room}: {count} files");
        }
    }
    println!("\n  {}", summary.next_hint);
    println!("\n{}", "=".repeat(55));
    println!();
}

fn print_doctor_human(summary: &mempalace_rs::model::DoctorSummary) {
    print!("{}", render_doctor_human(summary));
}

fn render_doctor_human(summary: &mempalace_rs::model::DoctorSummary) -> String {
    let mut out = String::new();
    out.push_str(&format!("\n{}\n", "=".repeat(55)));
    out.push_str("  MemPalace Doctor\n");
    out.push_str(&format!("{}\n\n", "=".repeat(55)));
    out.push_str(&format!("  Palace:     {}\n", summary.palace_path));
    out.push_str(&format!("  SQLite:     {}\n", summary.sqlite_path));
    out.push_str(&format!("  LanceDB:    {}\n", summary.lance_path));
    out.push_str(&format!("  Provider:   {}\n", summary.provider));
    out.push_str(&format!("  Model:      {}\n", summary.model));
    out.push_str(&format!("  Dimension:  {}\n", summary.dimension));
    if let Some(path) = &summary.cache_dir {
        out.push_str(&format!("  Cache dir:  {path}\n"));
    }
    if let Some(path) = &summary.model_cache_dir {
        out.push_str(&format!("  Model dir:  {path}\n"));
    }
    if let Some(path) = &summary.expected_model_file {
        out.push_str(&format!("  Model file: {path}\n"));
    }
    out.push_str(&format!(
        "  Cache hit:  {}\n",
        if summary.model_cache_present {
            "yes"
        } else {
            "no"
        }
    ));
    out.push_str(&format!(
        "  Model file present: {}\n",
        if summary.expected_model_file_present {
            "yes"
        } else {
            "no"
        }
    ));
    if let Some(path) = &summary.ort_dylib_path {
        out.push_str(&format!("  ORT dylib:  {path}\n"));
    }
    if let Some(endpoint) = &summary.hf_endpoint {
        out.push_str(&format!("  HF endpoint: {endpoint}\n"));
    }
    if !summary.model_cache_present {
        out.push_str("  Cache state: model cache directory not populated yet\n");
    } else if !summary.expected_model_file_present {
        out.push_str("  Cache state: model snapshot exists but onnx/model.onnx is missing\n");
    } else {
        out.push_str("  Cache state: model snapshot looks ready\n");
    }
    if summary.warmup_attempted {
        out.push_str(&format!(
            "  Warmup:     {}\n",
            if summary.warmup_ok { "ok" } else { "failed" }
        ));
        if let Some(error) = &summary.warmup_error {
            out.push_str(&format!("  Warmup err: {error}\n"));
        }
        if !summary.warmup_ok {
            out.push_str("\n  Suggested next step:\n");
            if summary.hf_endpoint.is_none() {
                out.push_str(
                    "    Retry with --hf-endpoint https://hf-mirror.com if the default HuggingFace route is blocked.\n",
                );
            } else {
                out.push_str(
                    "    Retry prepare-embedding after verifying the configured HuggingFace mirror and local network access.\n",
                );
            }
        }
    }
    out.push_str(&format!("\n{}\n\n", "=".repeat(55)));
    out
}

fn print_prepare_embedding_human(summary: &mempalace_rs::model::PrepareEmbeddingSummary) {
    print!("{}", render_prepare_embedding_human(summary));
}

fn render_prepare_embedding_human(
    summary: &mempalace_rs::model::PrepareEmbeddingSummary,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("\n{}\n", "=".repeat(55)));
    out.push_str("  MemPalace Prepare Embedding\n");
    out.push_str(&format!("{}\n\n", "=".repeat(55)));
    out.push_str(&format!("  Palace:    {}\n", summary.palace_path));
    out.push_str(&format!("  Provider:  {}\n", summary.provider));
    out.push_str(&format!("  Model:     {}\n", summary.model));
    out.push_str(&format!("  Attempts:  {}\n", summary.attempts));
    out.push_str(&format!(
        "  Result:    {}\n",
        if summary.success { "ok" } else { "failed" }
    ));
    if let Some(error) = &summary.last_error {
        out.push_str(&format!("  Last err:  {error}\n"));
    }
    out.push_str(&format!(
        "  Warmup:    {}\n",
        if summary.doctor.warmup_ok {
            "ok"
        } else {
            "failed"
        }
    ));
    if let Some(path) = &summary.doctor.model_cache_dir {
        out.push_str(&format!("  Model dir: {path}\n"));
    }
    if let Some(path) = &summary.doctor.expected_model_file {
        out.push_str(&format!("  Model file: {path}\n"));
    }
    out.push_str(&format!(
        "  Model file present: {}\n",
        if summary.doctor.expected_model_file_present {
            "yes"
        } else {
            "no"
        }
    ));
    if !summary.success {
        out.push_str("\n  Suggested next step:\n");
        if summary.doctor.hf_endpoint.is_none() {
            out.push_str(
                "    Retry with --hf-endpoint https://hf-mirror.com if model download cannot reach HuggingFace.\n",
            );
        } else {
            out.push_str(
                "    Verify the configured HuggingFace mirror and rerun prepare-embedding once model download works.\n",
            );
        }
    }
    out.push_str(&format!("\n{}\n\n", "=".repeat(55)));
    out
}

fn print_unsupported_mine_mode(
    mode: &str,
    extract: &str,
    dir: &Path,
    human: bool,
) -> anyhow::Result<()> {
    if human {
        print_unsupported_mine_mode_human(mode, extract, dir);
        return Ok(());
    }
    let payload = json!({
        "error": "Unsupported mine mode",
        "hint": "Rust currently supports only `mempalace mine <dir>` project ingest. Conversation and general extraction modes are not implemented yet.",
        "mode": mode,
        "extract": extract,
        "project_path": dir.display().to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn print_unsupported_mine_mode_human(mode: &str, extract: &str, dir: &Path) {
    println!("\n{}", "=".repeat(55));
    println!("  MemPalace Mine");
    println!("{}\n", "=".repeat(55));
    println!("  Project:  {}", dir.display());
    println!("  Mode:     {mode}");
    println!("  Extract:  {extract}");
    println!();
    println!("  Conversation and general extraction are not implemented in Rust yet.");
    println!("  Supported today: mempalace-rs mine <dir>");
    println!();
    println!("  Suggested next step:");
    println!(
        "    Retry with --mode projects, or keep using the Python CLI for conversation mining."
    );
    println!("\n{}", "=".repeat(55));
    println!();
}

#[cfg(test)]
mod tests {
    use super::{render_doctor_human, render_prepare_embedding_human};
    use mempalace_rs::model::{DoctorSummary, PrepareEmbeddingSummary};

    fn failed_doctor_summary(hf_endpoint: Option<&str>) -> DoctorSummary {
        DoctorSummary {
            kind: "doctor".to_string(),
            palace_path: "/tmp/palace".to_string(),
            sqlite_path: "/tmp/palace/palace.sqlite3".to_string(),
            lance_path: "/tmp/palace/lance".to_string(),
            version: "0.1.0".to_string(),
            provider: "fastembed".to_string(),
            model: "MultilingualE5Small".to_string(),
            dimension: 384,
            cache_dir: Some("/tmp/cache".to_string()),
            model_cache_dir: Some("/tmp/cache/model".to_string()),
            model_cache_present: false,
            expected_model_file: Some("/tmp/cache/model/onnx/model.onnx".to_string()),
            expected_model_file_present: false,
            hf_endpoint: hf_endpoint.map(ToOwned::to_owned),
            ort_dylib_path: Some(
                "/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib".to_string(),
            ),
            warmup_attempted: true,
            warmup_ok: false,
            warmup_error: Some("Failed to retrieve onnx/model.onnx".to_string()),
        }
    }

    #[test]
    fn doctor_human_failure_suggests_mirror_when_default_endpoint_fails() {
        let output = render_doctor_human(&failed_doctor_summary(None));
        assert!(output.contains("Cache state: model cache directory not populated yet"));
        assert!(output.contains("Warmup:     failed"));
        assert!(output.contains("Suggested next step:"));
        assert!(output.contains("--hf-endpoint https://hf-mirror.com"));
    }

    #[test]
    fn prepare_embedding_human_failure_mentions_configured_mirror_when_present() {
        let doctor = failed_doctor_summary(Some("https://hf-mirror.example"));
        let output = render_prepare_embedding_human(&PrepareEmbeddingSummary {
            kind: "prepare_embedding".to_string(),
            palace_path: "/tmp/palace".to_string(),
            sqlite_path: "/tmp/palace/palace.sqlite3".to_string(),
            lance_path: "/tmp/palace/lance".to_string(),
            version: "0.1.0".to_string(),
            provider: "fastembed".to_string(),
            model: "MultilingualE5Small".to_string(),
            attempts: 1,
            success: false,
            last_error: Some("Failed to retrieve onnx/model.onnx".to_string()),
            doctor,
        });
        assert!(output.contains("Result:    failed"));
        assert!(output.contains("Last err:  Failed to retrieve onnx/model.onnx"));
        assert!(output.contains("Suggested next step:"));
        assert!(output.contains("Verify the configured HuggingFace mirror"));
    }
}
