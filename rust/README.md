# Rust Rewrite

This directory holds the in-progress Rust rewrite of MemPalace.

Current dependency direction:

- `chromadb` -> `lancedb` for the default local-first embedded vector store
- embeddings -> `fastembed` with runtime-loaded ONNX Runtime, plus a built-in hash fallback for tests and offline safety
- `pyyaml` -> `serde_yml`
- Python CLI stack -> `clap`
- Python logging -> `tracing` + `tracing-subscriber`

The initial Cargo manifest commits to the embedded path only:
LanceDB for local vector storage, plus SQLite for relational and knowledge-graph state.

Current first-phase support:

- `init`
- `mine` for project files
- `search`
- `migrate`
- `repair`
- `status`
- `doctor`
- `prepare-embedding`
- read-only MCP tools for `status`, `list_wings`, `list_rooms`, `get_taxonomy`, `search`
- provider-based embedding layer with batch document embedding
- SQLite schema version tracking and a minimal migration path
- `migrate` exposes the current SQLite schema upgrade path as a CLI command
- `repair` provides non-destructive palace diagnostics

Current project-mining behavior:

- reads `mempalace.yaml` or legacy `mempal.yaml` when present
- uses config-defined `wing` and `rooms`
- routes files to rooms using path, filename, and keyword scoring
- skips known generated/cache directories and non-readable extensions by default
- supports explicit `--include-ignored` paths for `.gitignore`d files

Embedding configuration:

- default provider: `fastembed`
- default model: `MultilingualE5Small`
- test/CI fallback: `MEMPALACE_RS_EMBED_PROVIDER=hash`

Useful env vars:

- `MEMPALACE_RS_EMBED_PROVIDER=fastembed|hash`
- `MEMPALACE_RS_EMBED_MODEL=MultilingualE5Small`
- `MEMPALACE_RS_EMBED_CACHE_DIR=/path/to/model-cache`
- `MEMPALACE_RS_HF_ENDPOINT=https://hf-mirror.com`
- `MEMPALACE_RS_EMBED_SHOW_DOWNLOAD_PROGRESS=true|false`

Useful verification command:

- `cargo run -- --palace /tmp/mempalace doctor`
- `cargo run -- --palace /tmp/mempalace doctor --warm-embedding`
- `cargo run -- --palace /tmp/mempalace prepare-embedding`
- `cargo run -- --palace /tmp/mempalace migrate`
- `cargo run -- --palace /tmp/mempalace repair`
- `cargo run -- --palace /tmp/mempalace --hf-endpoint https://hf-mirror.com prepare-embedding`
- `MEMPALACE_RS_TEST_HF_ENDPOINT=https://hf-mirror.com cargo test cli_fastembed_prepare_mine_search_smoke -- --ignored --nocapture`

Recommended first-run flow for fastembed:

1. `cargo run -- --palace /tmp/mempalace doctor`
2. `cargo run -- --palace /tmp/mempalace --hf-endpoint https://hf-mirror.com prepare-embedding --attempts 3 --wait-ms 1000`
3. `cargo run -- --palace /tmp/mempalace mine /path/to/project`

Fastembed smoke test note:

- the real `fastembed` CLI integration test is marked `ignored`
- run it explicitly when you want a true local `prepare-embedding -> mine -> search` pass
- set `MEMPALACE_RS_TEST_HF_ENDPOINT` if your environment needs a HuggingFace mirror

Current MCP compatibility notes:

- read-only MCP tool names match the Python server
- `tools/list` now exposes Python-style input schemas for read tools
- `mempalace_status` includes `protocol` and `aaak_dialect`
- `status` and MCP status now expose `schema_version`
- `mempalace_search` returns Python-style `query`, `filters`, `source_file`, and `similarity`
- empty palaces return the Python-style `{"error":"No palace found","hint":"Run: ..."}` shape

Local runtime note:

- `fastembed` now uses runtime-loaded ONNX Runtime instead of build-time binary downloads
- on macOS with Homebrew, install `onnxruntime`
- the code will auto-detect `/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib` and `/usr/local/opt/onnxruntime/lib/libonnxruntime.dylib`

Intentionally not in this first Rust phase:

- write MCP tools
- hooks
- AAAK generation
- conversation mining
- direct compatibility with Python palace data

Current repair scope:

- checks whether SQLite and LanceDB paths exist
- reports `schema_version`, embedding profile, and SQLite drawer count
- checks whether the current LanceDB table is accessible
- does not rebuild or delete any data
