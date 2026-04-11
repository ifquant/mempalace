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
- `status`
- `doctor`
- read-only MCP tools for `status`, `list_wings`, `list_rooms`, `get_taxonomy`, `search`
- provider-based embedding layer with batch document embedding

Embedding configuration:

- default provider: `fastembed`
- default model: `MultilingualE5Small`
- test/CI fallback: `MEMPALACE_RS_EMBED_PROVIDER=hash`

Useful env vars:

- `MEMPALACE_RS_EMBED_PROVIDER=fastembed|hash`
- `MEMPALACE_RS_EMBED_MODEL=MultilingualE5Small`
- `MEMPALACE_RS_EMBED_CACHE_DIR=/path/to/model-cache`
- `MEMPALACE_RS_EMBED_SHOW_DOWNLOAD_PROGRESS=true|false`

Useful verification command:

- `cargo run -- --palace /tmp/mempalace doctor`
- `cargo run -- --palace /tmp/mempalace doctor --warm-embedding`

Local runtime note:

- `fastembed` now uses runtime-loaded ONNX Runtime instead of build-time binary downloads
- on macOS with Homebrew, install `onnxruntime`
- the code will auto-detect `/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib` and `/usr/local/opt/onnxruntime/lib/libonnxruntime.dylib`

Intentionally not in this first Rust phase:

- write MCP tools
- hooks
- repair / migrate
- AAAK generation
- conversation mining
- direct compatibility with Python palace data
