use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::helper_cli::HookCommand;
use crate::palace_cli::RepairCommand;
use crate::registry_cli::RegistryCommand;

#[derive(Parser)]
#[command(name = "mempalace-rs")]
#[command(
    about = "MemPalace — Give your AI a memory. No API key required.",
    long_about = "MemPalace — Give your AI a memory. No API key required.\n\nCurrent Rust phase supports local-first project bootstrap, mining/search, transcript normalization and splitting, AAAK compression, wake-up and recall layers, registry workflows, maintenance diagnostics, and MCP tools.\n\nExamples:\n  mempalace-rs init ~/projects/my_app\n  mempalace-rs onboarding ~/projects/my_app --mode combo --scan\n  mempalace-rs mine ~/projects/my_app\n  mempalace-rs normalize ~/exports/chat.jsonl --human\n  mempalace-rs search \"why did we switch to GraphQL\"\n  mempalace-rs recall --wing my_app --room decisions\n  mempalace-rs registry summary ~/projects/my_app\n  mempalace-rs status"
)]
pub struct Cli {
    #[arg(long)]
    #[arg(
        help = "Where the palace lives (default: ~/.mempalace-rs/palace or MEMPALACE_RS_PALACE_PATH)"
    )]
    pub palace: Option<PathBuf>,
    #[arg(long)]
    #[arg(help = "Override the HuggingFace endpoint used by fastembed model downloads")]
    pub hf_endpoint: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Set up a palace directory for a project")]
    Init {
        #[arg(help = "Project directory to set up")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(
            help = "Auto-accept detected bootstrap files (Rust init is already non-interactive)"
        )]
        yes: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable init summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Guide first-run registry and AAAK bootstrap for a project")]
    Onboarding {
        #[arg(help = "Project directory to seed")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(help = "Usage mode: work, personal, or combo")]
        mode: Option<String>,
        #[arg(long = "person")]
        #[arg(help = "Seed person as name,relationship,context; repeat as needed")]
        people: Vec<String>,
        #[arg(long = "project")]
        #[arg(help = "Seed one project name; repeat as needed")]
        projects: Vec<String>,
        #[arg(long = "alias")]
        #[arg(help = "Seed alias mapping as alias=canonical; repeat as needed")]
        aliases: Vec<String>,
        #[arg(long)]
        #[arg(help = "Comma-separated wing list; defaults follow the selected mode")]
        wings: Option<String>,
        #[arg(long)]
        #[arg(help = "Scan local files for additional names before writing the registry")]
        scan: bool,
        #[arg(long)]
        #[arg(help = "Auto-accept detected names during scan")]
        auto_accept_detected: bool,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable onboarding summary instead of JSON")]
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
    #[command(about = "Split concatenated transcript mega-files into per-session files")]
    Split {
        #[arg(help = "Directory containing transcript files")]
        dir: PathBuf,
        #[arg(long)]
        #[arg(help = "Write split files here (default: same directory as source files)")]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = 2)]
        #[arg(help = "Only split files containing at least N sessions")]
        min_sessions: usize,
        #[arg(long)]
        #[arg(help = "Show what would be split without writing files")]
        dry_run: bool,
    },
    #[command(about = "Normalize one chat export into MemPalace transcript format")]
    Normalize {
        #[arg(help = "Chat export or transcript file to normalize")]
        file: PathBuf,
        #[arg(long)]
        #[arg(help = "Print human-readable preview instead of JSON")]
        human: bool,
    },
    #[command(about = "Compress drawers into AAAK summaries")]
    Compress {
        #[arg(long)]
        #[arg(help = "Limit compression to one project/wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Preview AAAK summaries without storing them")]
        dry_run: bool,
        #[arg(long)]
        #[arg(help = "Print human-readable compression summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Show L0 + L1 wake-up context")]
    WakeUp {
        #[arg(long)]
        #[arg(help = "Show wake-up context for one project/wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Print human-readable wake-up context instead of JSON")]
        human: bool,
    },
    #[command(about = "Recall stored drawers by wing/room without semantic search")]
    Recall {
        #[arg(long)]
        #[arg(help = "Limit recall to one project/wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Limit recall to one room")]
        room: Option<String>,
        #[arg(long, default_value_t = 10)]
        #[arg(help = "Maximum number of drawers to return")]
        results: usize,
        #[arg(long)]
        #[arg(help = "Print human-readable recall output instead of JSON")]
        human: bool,
    },
    #[command(about = "Show Layer 0-3 stack status")]
    LayersStatus {
        #[arg(long)]
        #[arg(help = "Print human-readable layer status instead of JSON")]
        human: bool,
    },
    #[command(about = "Upgrade palace SQLite metadata to the current schema version")]
    Migrate {
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable migration summary instead of JSON")]
        human: bool,
    },
    #[command(about = "Run repair diagnostics or repair subcommands")]
    Repair {
        #[command(subcommand)]
        action: Option<RepairCommand>,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable repair diagnostics instead of JSON")]
        human: bool,
    },
    #[command(about = "Deduplicate near-identical drawers")]
    Dedup {
        #[arg(long, default_value_t = 0.15)]
        #[arg(help = "Cosine distance threshold (lower = stricter)")]
        threshold: f64,
        #[arg(long)]
        #[arg(help = "Preview without deleting")]
        dry_run: bool,
        #[arg(long)]
        #[arg(help = "Show stats only")]
        stats: bool,
        #[arg(long)]
        #[arg(help = "Scope dedup to one wing")]
        wing: Option<String>,
        #[arg(long)]
        #[arg(help = "Filter by source file pattern")]
        source: Option<String>,
        #[arg(long)]
        #[arg(help = "Print Python-style human-readable dedup summary instead of JSON")]
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
    #[command(about = "Run hook logic (reads JSON from stdin, outputs JSON to stdout)")]
    Hook {
        #[command(subcommand)]
        action: HookCommand,
    },
    #[command(about = "Output skill instructions to stdout")]
    Instructions {
        #[arg(help = "Instruction set name")]
        name: String,
    },
    #[command(about = "Inspect and update the project-local entity registry")]
    Registry {
        #[command(subcommand)]
        action: RegistryCommand,
    },
    #[command(about = "Show MCP setup help or run the read-only MCP server")]
    Mcp {
        #[arg(long)]
        #[arg(help = "Print Python-style MCP setup instructions")]
        setup: bool,
        #[arg(long)]
        #[arg(help = "Run the MCP server on stdio instead of printing setup help")]
        serve: bool,
    },
}
