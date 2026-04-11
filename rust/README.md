# Rust Rewrite

This directory holds the in-progress Rust rewrite of MemPalace.

Current dependency direction:

- `chromadb` -> `lancedb` for the default local-first embedded vector store
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
- read-only MCP tools for `status`, `list_wings`, `list_rooms`, `get_taxonomy`, `search`

Intentionally not in this first Rust phase:

- write MCP tools
- hooks
- repair / migrate
- AAAK generation
- conversation mining
- direct compatibility with Python palace data
