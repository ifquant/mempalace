# Rust Rewrite

This directory holds the future Rust rewrite of MemPalace.

Current dependency direction:

- `chromadb` -> `lancedb` for the default local-first embedded vector store
- `pyyaml` -> `serde_yml`
- Python CLI stack -> `clap`
- Python logging -> `tracing` + `tracing-subscriber`

The initial Cargo manifest is intentionally thin and currently commits to the embedded path only:
LanceDB for local vector storage, plus SQLite for relational and knowledge-graph state.
