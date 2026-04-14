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
- `mine --dry-run` to preview file discovery and chunk counts without persisting drawers
- `mine --mode projects --agent <name>` matches more of the Python CLI surface
- `mine --mode convos` and `--extract general` are accepted at the CLI boundary but currently return a structured "not implemented yet" response
- project re-mine bookkeeping now tracks `source_mtime` so unchanged files skip more like the Python miner
- `mine` JSON now carries per-room file counts plus the Python-style search follow-up hint
- project scanning now matches more Python `scan_project()` edge cases around nested `.gitignore`, negation, and include-overrides for skipped directories
- SQLite drawer records now persist Python-style project metadata: `source_file`, `source_mtime`, `added_by`, and `filed_at`
- `mine` JSON now also carries Python-style header context: configured room names and the planned file count after applying `--limit`
- `mine --progress` prints Python-style per-file progress to `stderr` while keeping the final JSON summary on `stdout`
- LanceDB drawer rows now persist Python-style metadata too: `source_file`, `source_mtime`, `added_by`, `filed_at`
- legacy LanceDB tables are upgraded in place with those metadata columns during `init`/`mine`/`search`
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
- CLI help and command descriptions are now closer to the Python entrypoint
- `search` CLI JSON now carries Python-style `query`, `filters`, `source_file`, and `similarity`
- `search` CLI JSON now also exposes vector-backed `source_mtime`, `added_by`, and `filed_at`
- `search` now normalizes `source_file` to a basename, rounds `similarity` to 3 decimals, and keeps duplicate chunks as separate hits like Python
- `search --human` prints a Python-style readable result view while the default CLI output stays JSON
- `search --human` now also uses Python-style text for the no-palace path instead of JSON
- `search --human` also prints `Search error: ...` text when the search fails after palace startup checks
- default JSON `search` now also returns a structured `{"error":"Search error: ..."}` payload on query-time failures
- `mempalace_search` in the MCP server now also returns tool-level `{"error":"Search error: ..."}` content instead of a transport error on query-time failures
- `status`, `migrate`, and `repair` now carry stable `kind`/path/version context fields
- `status --human` prints a Python-style readable palace summary while the default CLI output stays JSON
- `status --human` now also explains when the palace exists but is still empty, with a direct `mempalace mine <dir>` next step
- `status --human` now also formats execution-time failures, such as a broken SQLite file, into readable status-specific text before exiting non-zero
- `repair --human` prints a Python-style readable diagnostics summary while the default CLI output stays JSON
- `repair --human` now also formats execution-time failures, such as a broken SQLite file, into readable repair-specific text before exiting non-zero
- `migrate --human` prints a Python-style readable migration summary while the default CLI output stays JSON
- `migrate --human` now also formats execution-time failures, such as a broken SQLite file, into readable migrate-specific text before exiting non-zero
- `init --human` prints a Python-style readable init summary while the default CLI output stays JSON
- `init --human` now also formats execution-time failures, such as a broken SQLite file already present under the palace path, into readable init-specific text before exiting non-zero
- `doctor --human` prints a Python-style readable embedding diagnostics summary while the default CLI output stays JSON, including cache-state conclusions and a suggested next step when warm-up fails
- `prepare-embedding --human` prints a Python-style readable embedding preparation summary while the default CLI output stays JSON, including a suggested next step when model warm-up still fails
- `doctor --human` and `prepare-embedding --human` now also format invalid embedding-provider failures into readable command-specific text before exiting non-zero
- `mine --human` prints a Python-style readable mine summary while the default CLI output stays JSON and `--progress` keeps using stderr
- `mine --human --mode convos` now also fails with a readable text hint instead of a JSON blob, while the default unsupported-mode path stays JSON
- `mine --human` now also explains when project scanning found no matching files, instead of only showing zero counts
- `mine --human --dry-run` now makes it explicit that the run was preview-only, labels drawer counts as previewed, and states that nothing was written
- `init` and `mine` now also carry stable `kind`/path/version context fields
- `doctor` and `prepare-embedding` now also carry stable `kind`/path/version context fields
- CLI `status/search` now return Python-style `error + hint` when no palace exists

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
