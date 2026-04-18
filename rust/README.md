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
- `init` now bootstraps project-local `mempalace.yaml` room config and writes `entities.json` when it can confidently detect people/projects from local prose files
- `init` now also writes palace-ready `aaak_entities.md` and `critical_facts.md` bootstrap docs next to the project when those files do not already exist
- `mine` for project files
- `mine --dry-run` to preview file discovery and chunk counts without persisting drawers
- `mine --mode projects --agent <name>` matches more of the Python CLI surface
- `mine --mode convos --extract exchange` now mines normalized chat transcripts into exchange-pair drawers
- `mine --mode convos --extract general` now mines 5 heuristic memory types: `decision`, `preference`, `milestone`, `problem`, `emotional`
- convos mode now supports `.txt/.md/.json/.jsonl`, including normalized ChatGPT JSON and Codex/Claude-style JSONL exports
- convos mode skips `.meta.json`, symlinks, oversized files, and unsupported/broken chat exports without aborting the batch
- convos re-mine now follows the same source-based replacement path as project mining, so re-filing one chat file replaces old chunks instead of duplicating them
- drawer metadata now also persists `ingest_mode` and `extract_mode` in both SQLite and LanceDB
- exchange mode now has explicit coverage for quoted turns, speaker-turn transcripts, and paragraph fallback
- general mode now has explicit coverage for keeping positive emotional text out of the `problem` bucket
- project re-mine bookkeeping now tracks `source_mtime` so unchanged files skip more like the Python miner
- `mine` JSON now carries per-room file counts plus the Python-style search follow-up hint
- project scanning now matches more Python `scan_project()` edge cases around nested `.gitignore`, negation, and include-overrides for skipped directories
- SQLite drawer records now persist Python-style project metadata: `source_file`, `source_mtime`, `added_by`, and `filed_at`
- `mine` JSON now also carries Python-style header context: configured room names and the planned file count after applying `--limit`
- `mine --progress` prints Python-style per-file progress to `stderr` while keeping the final JSON summary on `stdout`
- LanceDB drawer rows now persist Python-style metadata too: `source_file`, `source_mtime`, `added_by`, `filed_at`
- legacy LanceDB tables are upgraded in place with those metadata columns during `init`/`mine`/`search`
- `search`
- `compress` to generate and persist AAAK summaries for existing drawers
- `wake-up` to render palace-local `identity.txt` plus an L1 essential-story summary
- `hook run --hook session-start|stop|precompact --harness claude-code|codex` for harness-side auto-save integration
- `instructions <help|init|mine|search|status>` to print built-in skill guidance markdown
- `split <dir>` to detect transcript mega-files, preview session boundaries, and split them into per-session `.txt` files with `.mega_backup` rollover
- `migrate`
- `repair`
- `status`
- `doctor`
- `prepare-embedding`
- read-only MCP tools for `status`, `list_wings`, `list_rooms`, `get_taxonomy`, `search`
- read-only MCP tools now also include `check_duplicate` and `get_aaak_spec`, matching more of the Python MCP surface
- read-only MCP tools now also include the room-graph trio: `traverse`, `find_tunnels`, `graph_stats`
- read-only MCP tools now also include the KG read trio: `kg_query`, `kg_timeline`, `kg_stats`
- MCP now also includes the first diary write/read surface: `diary_write`, `diary_read`
- MCP now also includes the first Python-style write surface: `add_drawer`, `delete_drawer`, `kg_add`, `kg_invalidate`
- MCP write tools now append a palace-local JSONL write-ahead log under `palace/wal/write_log.jsonl`
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
- `search --human` and `mine --human` now also format invalid embedding-provider failures into readable command-specific text before exiting non-zero
- `search` and `mine` now also emit structured JSON errors for invalid embedding-provider failures by default, while their `--human` variants keep command-specific readable text
- `search` and `mine` also have regression coverage for broken SQLite execution failures on their default JSON surface
- `search --human` and `mine --human` also have regression coverage for broken SQLite execution failures on their readable text surface
- default JSON `search` now also returns a structured `{"error":"Search error: ..."}` payload on query-time failures
- `mempalace_search` in the MCP server now also returns tool-level `{"error":"Search error: ..."}` content instead of a transport error on query-time failures
- read-only MCP tools now consistently keep execution failures inside tool content with `error + hint`, instead of escalating broken-palace reads into JSON-RPC transport errors
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
- `init`, `status`, `repair`, and `migrate` now also emit structured JSON errors for invalid embedding-provider failures by default, while their `--human` variants keep command-specific readable text
- structured CLI error payloads now consistently include both `error` and `hint` across `init/search/mine/status/repair/migrate/doctor/prepare-embedding`
- `init --human`, `status --human`, `repair --human`, and `migrate --human` now all have explicit regression coverage for invalid embedding-provider failures
- `init`, `status`, `repair`, and `migrate` now also emit structured JSON errors for broken SQLite execution failures by default, instead of falling back to raw stderr
- `doctor --human` prints a Python-style readable embedding diagnostics summary while the default CLI output stays JSON, including cache-state conclusions and a suggested next step when warm-up fails
- `prepare-embedding --human` prints a Python-style readable embedding preparation summary while the default CLI output stays JSON, including a suggested next step when model warm-up still fails
- `doctor` and `prepare-embedding` now also emit structured JSON errors for invalid embedding-provider failures by default, while their `--human` variants keep command-specific readable text
- `mine --human` prints a Python-style readable mine summary while the default CLI output stays JSON and `--progress` keeps using stderr
- `mine --human --mode convos` now prints a readable conversation-mining summary instead of falling back to an unsupported-mode hint
- `mine --human` now also explains when project scanning found no matching files, instead of only showing zero counts
- `mine --human --dry-run` now makes it explicit that the run was preview-only, labels drawer counts as previewed, and states that nothing was written
- `init` and `mine` now also carry stable `kind`/path/version context fields
- `doctor` and `prepare-embedding` now also carry stable `kind`/path/version context fields
- CLI `status/search` now return Python-style `error + hint` when no palace exists

Current project-mining behavior:

- `init` bootstraps `mempalace.yaml` from folder structure / filename patterns when the file does not already exist
- `init` preserves existing `mempalace.yaml` and `entities.json` instead of overwriting them
- `init` also preserves existing `aaak_entities.md` and `critical_facts.md` instead of overwriting them
- reads `mempalace.yaml` or legacy `mempal.yaml` when present
- uses config-defined `wing` and `rooms`
- skips init-generated bootstrap artifacts such as `entities.json`, `aaak_entities.md`, and `critical_facts.md` during normal project mining
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
4. `cargo run -- --palace /tmp/mempalace mine /path/to/chats --mode convos --extract exchange`
5. `cargo run -- --palace /tmp/mempalace mine /path/to/chats --mode convos --extract general`

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
- `mempalace_check_duplicate` now returns Python-style `is_duplicate` and `matches[*].id/wing/room/similarity/content`
- `mempalace_get_aaak_spec` now exposes the standalone AAAK dialect text like the Python server
- `mempalace_traverse`, `mempalace_find_tunnels`, and `mempalace_graph_stats` now expose a Python-style room graph built from Rust drawer metadata
- `mempalace_kg_query`, `mempalace_kg_timeline`, and `mempalace_kg_stats` now expose a Python-style temporal KG read surface built from Rust SQLite triples
- `mempalace_kg_add` and `mempalace_kg_invalidate` now expose Python-style KG write operations with structured success payloads
- `mempalace_add_drawer` and `mempalace_delete_drawer` now expose Python-style drawer write/delete operations backed by Rust SQLite + LanceDB
- `mempalace_diary_write` and `mempalace_diary_read` now expose a Python-style agent diary surface backed by Rust SQLite
- write MCP tools now append audit entries before execution to `palace/wal/write_log.jsonl`, keeping Rust's local-first data under the palace root instead of a global home-level WAL path
- empty palaces return the Python-style `{"error":"No palace found","hint":"Run: ..."}` shape
- execution failures in MCP tools now also return tool-level `{"error":"...","hint":"..."}` payloads instead of escalating transport errors
- `mempalace_add_drawer` and `mempalace_kg_add` can now auto-bootstrap a new Rust palace, matching the Python write-first workflow more closely
- `compress` stores AAAK output in SQLite table `compressed_drawers`, keeping the summary layer local to the palace without introducing a second external backend
- `wake-up` reads identity from `<palace>/identity.txt` instead of the Python global `~/.mempalace/identity.txt`, keeping Rust's local-first palace self-contained
- hook state now lives under `<palace>/hook_state/` instead of Python's global `~/.mempalace/hook_state/`, keeping auto-save bookkeeping local to the active palace
- `split` follows the Python mega-file workflow: detect true `Claude Code v` session starts, skip context-restore headers, and rename the original transcript to `.mega_backup` after a successful split

Local runtime note:

- `fastembed` now uses runtime-loaded ONNX Runtime instead of build-time binary downloads
- on macOS with Homebrew, install `onnxruntime`
- the code will auto-detect `/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib` and `/usr/local/opt/onnxruntime/lib/libonnxruntime.dylib`

Intentionally not in this first Rust phase:

- the remaining Python write MCP surface beyond drawer/KG/diary basics
- direct compatibility with Python palace data

Current repair scope:

- checks whether SQLite and LanceDB paths exist
- reports `schema_version`, embedding profile, and SQLite drawer count
- checks whether the current LanceDB table is accessible
- does not rebuild or delete any data
