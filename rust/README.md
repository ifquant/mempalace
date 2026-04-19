# Rust Rewrite

This directory holds the in-progress Rust rewrite of MemPalace.

Current parity status:

- the Rust rewrite now covers the current public Python CLI surface and the current public Python MCP tool surface
- the top-level Rust CLI help now reflects the broader current surface: bootstrap, transcript prep, recall layers, registry workflows, maintenance, and MCP
- detailed Rust/Python parity tracking now lives in [`../docs/parity-ledger.md`](../docs/parity-ledger.md)
- remaining work is mainly truth-in-docs, help/test consistency, and deeper non-CLI behavior audits rather than large missing user-facing commands

Current dependency direction:

- `chromadb` -> `lancedb` for the default local-first embedded vector store
- embeddings -> `fastembed` with runtime-loaded ONNX Runtime, plus a built-in hash fallback for tests and offline safety
- `pyyaml` -> `serde_yml`
- Python CLI stack -> `clap`
- Python logging -> `tracing` + `tracing-subscriber`

The initial Cargo manifest commits to the embedded path only:
LanceDB for local vector storage, plus SQLite for relational and knowledge-graph state.

Rust library structure now also includes a shared `palace` module for reusable storage-facing helpers such as:

- default skip-directory policy shared with project mining
- vector-store bootstrap for library/script callers
- `file_already_mined` / source-state helpers mirroring Python `palace.py`

Rust library structure now also includes a `layers` module with a programmatic `LayerStack`
facade mirroring Python `layers.py`:

- `layer0()` for identity text
- `layer1()` for the essential story view
- `wake_up()` / `recall()` / `search()` / `status()` for the higher-level stack surfaces
- shared `read_identity_text()` / `render_layer1()` / `render_layer2()` helpers for Python-style layer text rendering

Rust library structure now also includes a `searcher` module with a programmatic search facade
mirroring Python `searcher.py`:

- `Searcher::search()` for programmatic Layer 3 retrieval
- `render_search_human()` for Python-style human-readable result blocks
- `normalize_search_hits()` / `normalize_source_file()` for Python-style result normalization and ordering

Rust library structure now also includes an `entity_detector` module mirroring Python
`entity_detector.py`:

- `detect_entities()` for project-local people/project detection
- `detect_entities_for_registry()` for registry/bootstrap callers
- `scan_for_detection()` for reusable file discovery before detection
- entity detector internals are now also split by concern:
  - `entity_detector_scan` for detection-time file discovery and noise-directory filtering
  - `entity_detector_score` for stopwords plus person/project scoring heuristics
  - `entity_detector` itself now stays as the thin public facade around candidate extraction and final ranking

Rust library structure now also includes a `room_detector` module mirroring Python
`room_detector_local.py`:

- `detect_rooms()` for project-local room bootstrap from folder/file signals
- `load_project_rooms()` for reading `mempalace.yaml` / `mempal.yaml`
- `detect_room()` for shared project-mining room routing
- room detector internals are now also split by concern:
  - `room_detector_config` for project config loading and default-room fallback
  - `room_detector_detect` for folder/file heuristics and content-based room routing
  - `room_detector` itself now stays as the thin public facade around shared room types

Rust library structure now also includes a `normalize` module mirroring Python
`normalize.py`:

- `normalize_conversation_file()` for file-based transcript normalization
- `normalize_conversation()` for library callers that already hold raw content
- shared JSON/JSONL chat-export parsing for ChatGPT, Claude, Codex, and Slack-style inputs

Rust normalize internals are now also split by concern instead of keeping quote
transcript handling and all chat-export parsers in one file:

- `normalize_transcript` for quote-line detection and transcript assembly/spellcheck helpers
- `normalize_json_jsonl` for Claude Code / Codex-style JSONL exports
- `normalize_json_exports` for ChatGPT / Claude / Slack / flat JSON export parsing plus shared content extraction
- `normalize_json` itself now stays as the thin facade over JSON/JSONL export routing
- spellcheck internals are now also split by concern:
  - `spellcheck_dict` for common-typo maps, system dictionary loading, edit distance, and candidate ranking
  - `spellcheck_rules` for token skip rules and regex-based technical/name/code guards
  - `spellcheck` itself now stays focused on transcript/user-text entrypoints and nearby registry-name loading
- `normalize` itself now stays focused on the public normalization entrypoints and top-level routing

Rust library structure now also includes a `palace_graph` module mirroring Python
`palace_graph.py`:

- `build_room_graph()` for graph construction from Rust drawer metadata
- `traverse_graph()` for BFS traversal from one room
- `find_tunnels()` and `graph_stats()` for cross-wing bridge discovery and summary stats

Rust library structure now also includes a `knowledge_graph` module mirroring Python
`knowledge_graph.py`:

- `KnowledgeGraph::add_triple()` / `invalidate()` for temporal fact writes
- `KnowledgeGraph::query_entity()` / `timeline()` for read-side traversal
- `KnowledgeGraph::stats()` for summary counts and relationship-type inspection

Rust library structure now also includes an `mcp_schema` module so the MCP
catalog/protocol surface no longer lives inline inside `mcp.rs`:

- `SUPPORTED_PROTOCOL_VERSIONS` and `PALACE_PROTOCOL` for MCP handshake/protocol metadata
- `tools()` for the MCP tool catalog and input schemas
- shared argument coercion / required-arg helpers for MCP tool calls

Rust library structure now also includes an `mcp_runtime` module so the MCP
tool execution surface no longer lives inline inside `mcp.rs`:

- `call_tool()` for the full MCP tool dispatch/runtime path
- shared tool-level `error + hint` formatting for MCP responses
- shared best-effort palace-local WAL logging for write-side MCP tools

Rust MCP runtime is now also split by tool family, instead of keeping every
tool execution branch in one giant match:

- `mcp_runtime_read` for palace read-side, graph, and KG read tools
- `mcp_runtime_write` for write-side, maintenance, and diary write tools
- `mcp_runtime_project` for onboarding, normalize, split, instructions, and hook helpers
- `mcp_runtime_registry` for project-local entity registry tools

Rust MCP schema/catalog is now also split by tool family instead of keeping
every tool schema in one giant list:

- `mcp_schema_catalog_read` for palace read-side, graph, and KG read tool schemas
- `mcp_schema_catalog_write` for write-side, maintenance, and diary write tool schemas
- `mcp_schema_catalog_project` for onboarding, normalize, split, instructions, and hook schemas
- `mcp_schema_catalog_registry` for project-local entity registry tool schemas
- `mcp_schema_support` for protocol negotiation, no-palace policy, argument coercion, and shared MCP helper functions

Rust library structure now also includes a `dedup` module mirroring Python
`dedup.py`:

- `Deduplicator::plan()` for grouping same-source drawers and identifying duplicates
- `DedupPlan::into_summary()` for turning a plan into the CLI/MCP summary payload
- shared cosine-distance logic for vector-level duplicate detection

Rust library structure now also includes a `repair` module mirroring Python
`repair.py`:

- `RepairContext` for shared repair path/version context and summary builders
- `read_corrupt_ids()` / `backup_sqlite_source()` for the prune/rebuild filesystem path
- shared scan/prune/rebuild summary assembly outside `service`

Rust library structure now also includes a `drawers` module for reusable drawer-write
helpers shared by MCP/manual filing and rebuild paths:

- `build_manual_drawer()` for Python-style manual drawer IDs and metadata
- `drawer_input_from_record()` for converting SQLite drawer rows back into write inputs
- shared `sanitize_name()` validation for drawer/KG write surfaces

Rust library structure now also includes a `compress` module for reusable AAAK compression
planning shared by CLI and future library callers:

- `CompressionRun::from_drawers()` for building `CompressedDrawer` entries plus token totals
- `CompressSummaryContext` for stable CLI/MCP summary assembly
- shared conversion from `DrawerRecord` into persisted AAAK rows

Rust library structure now also includes a `compression_runtime` module for
AAAK compression orchestration around SQLite persistence:

- `CompressionRuntime::compress()` for the read-drawers -> build-plan -> optional-persist flow
- shared SQLite/bootstrap handling so `service` no longer owns the compression path directly

Rust library structure now also includes an `embedding_runtime` module for reusable
doctor/prepare-embedding orchestration around the embedder:

- `EmbeddingRuntime::doctor()` and `EmbeddingRuntime::prepare_embedding()` as the facade used by `service`
- `finalize_doctor_summary()` for filling stable path/version fields
- `prepare_embedding_run()` for warmup retry flow and result capture
- `EmbeddingRuntimeContext` for stable doctor/prepare summary assembly

Rust library structure now also includes a `miner` module mirroring the Python
`miner.py` orchestration boundary:

- `mine_project_run()` for project-file ingest orchestration
- `mine_conversations_run()` for convo/general ingest orchestration
- shared file discovery, chunking, include-override, and conversation drawer assembly helpers

Rust library structure now also includes a `palace_read` module for read-side
palace surfacing across CLI, MCP, and library callers:

- `status()` / `list_wings()` / `list_rooms()` / `taxonomy()` for palace summary reads
- `traverse_graph()` / `find_tunnels()` / `graph_stats()` for graph-oriented read flows
- `search()` / `wake_up()` / `recall()` / `layer_status()` for the higher-level read surfaces

Rust library structure now also includes a `registry_runtime` module for
project-local entity registry orchestration:

- `summary()` / `lookup()` / `query()` for project-bound registry reads
- `learn()` for re-running entity detection and persisting new registry entries
- `add_person()` / `add_project()` / `add_alias()` / `research()` / `confirm_research()` for write-side project registry flows

Rust registry internals are now also split by concern instead of keeping data
types, heuristic lookup, and Wikipedia research in one giant file:

- `registry_types` for registry data structures, seed types, and shared constants
- `registry_research` for Wikipedia lookup and classification heuristics
- `registry` itself now stays focused on `EntityRegistry` behavior, disambiguation, and query/learn flows while re-exporting the public registry types

Rust conversation ingestion internals are now also split by concern instead of
keeping scan, exchange chunking, and general-memory extraction in one file:

- `convo_scan` for filesystem scanning, include overrides, and conversation file filtering
- `convo_scan_include` for include-override path normalization and force-include matching
- `convo_scan_walk` for ignore-aware filesystem walking and conversation file skip rules
- `convo_scan` itself now stays focused on the thin scan facade and scan-related test anchors
- `convo_exchange_rooms` for exchange room buckets and room-detection keyword routing
- `convo_exchange_chunking` for quote-line, speaker-turn, and paragraph-based exchange chunking
- `convo_exchange` itself now stays focused on the thin exchange facade and public chunk extraction entrypoint
- `convo_general` for prose extraction, marker scoring, sentiment/disambiguation, and general-memory extraction
- `convo` itself now stays a thin facade over the public conversation API and shared `ConversationChunk` type

Rust general-memory extraction internals are now also split by concern instead
of keeping segmentation and scoring heuristics in one file:

- `convo_general_segments` for transcript segmentation, paragraph fallback, turn grouping, and prose extraction
- `convo_general_scoring` for marker scoring, confidence, sentiment, and resolution/disambiguation heuristics
- `convo_general` itself now stays focused on the public extraction loop and `ConversationChunk` assembly

Rust mining orchestration internals are now also split by concern instead of
keeping project mining, convo mining, and shared file/chunk helpers in one file:

- `miner_project` for project/code/document mining execution
- `miner_convo` for conversation/chat-export mining execution
- `miner_support` for shared discovery, chunking, slugging, and conversation drawer assembly helpers
- `miner` itself now stays a thin facade over the public mining entrypoints

Rust bootstrap internals are now also split by concern instead of keeping file
parsing/writing and generated bootstrap docs in one file:

- `bootstrap_files` for config/entities load-save helpers
- `bootstrap_docs` for `entity_registry.json`, `aaak_entities.md`, and `critical_facts.md` generation helpers
- `bootstrap` itself now stays focused on bootstrap orchestration and result assembly

Rust onboarding internals are now also split by concern instead of keeping
interactive prompts, request normalization, and auto-detection merge logic in one file:

- `onboarding_prompt` for interactive onboarding questions and terminal UI helpers
- `onboarding_support` for mode normalization, dedupe/merge helpers, and shared parse helpers
- `onboarding` itself now stays focused on onboarding orchestration and summary assembly

Rust registry internals are now also split one step further so lookup/query
heuristics no longer live in the same file as persistence and mutation flows:

- `registry_lookup` for ambiguous-name disambiguation, registry lookup, and query-side extraction helpers
- `registry_io` for load/save, summary, research cache access, and onboarding/bootstrap seeding
- `registry_mutation` for learn/add/alias/confirm flows plus ambiguous-flag recomputation
- `registry_research` for Wikipedia lookup and research classification heuristics
- `registry_types` for registry data structures and shared constants
- `registry` itself now stays as the thin public facade and test anchor for the registry surface

Rust model definitions are now also split by domain instead of keeping every
DTO, summary payload, and request type in one giant `model.rs`:

- `model_palace` for drawers, search, status, taxonomy, and graph-facing data types
- `model_ops` for KG, diary, and manual drawer write/read payloads
- `model_project` for mining, init, and onboarding request/summary types
- `model_runtime` for migrate/repair/dedup/compress/embed/layer runtime summaries
- `model_registry` for project-local entity registry result payloads
- `model` itself now stays a thin facade that re-exports the public model surface

Rust service-layer orchestration is now also split by capability family instead
of keeping every `App` method implementation inline in one file:

- `service_project` for init, project bootstrap, mining, and compression entrypoints
- `service_read` for palace read-side, graph, search, and layer-facing entrypoints
- `service_ops` for KG, diary, and manual drawer operations
- `service_registry` for project-local entity registry entrypoints
- `service_maintenance` for migrate, repair, dedup, and embedding-runtime entrypoints
- `service` itself now stays focused on `App` construction plus shared tests

Rust library structure now also includes a `palace_ops` module for project-local
manual palace operations across diary, KG, and manual drawer surfaces:

- `kg_add()` / `kg_invalidate()` plus raw/timeline/stats query helpers
- `add_drawer()` / `delete_drawer()` for SQLite + LanceDB manual filing flows
- `diary_write()` / `diary_read()` for palace-local diary persistence

Rust library structure now also includes a `maintenance_runtime` module for
palace migration, repair, rebuild, and dedup orchestration:

- `migrate()` for schema/runtime upgrade flow
- `repair()` / `repair_scan()` / `repair_prune()` / `repair_rebuild()` for maintenance and recovery flows
- `dedup()` for SQLite + LanceDB duplicate-cleanup orchestration around the lower-level dedup planner

Rust library structure now also includes an `init_runtime` module for palace
bootstrap and project bootstrap orchestration:

- `prepare_storage()` for the shared SQLite + vector bootstrap path
- `init()` for palace-local initialization summaries
- `init_project()` for project bootstrap plus world-file summary assembly

Rust CLI structure now also includes a `registry_cli` module so the registry command
surface no longer lives inline inside `main.rs`:

- `handle_registry_command()` for the registry subcommand dispatch path
- shared human-readable registry renderers kept next to the registry command wiring

Rust registry CLI is now also split by command family instead of keeping read,
write, research, and bootstrap helpers in one file:

- `registry_cli_read` for `summary`, `lookup`, `learn`, and `query`
- `registry_cli_write` for `add-person`, `add-project`, and `add-alias`
- `registry_cli_research` for `research` and `confirm`
- `registry_cli_support` for shared app/bootstrap and JSON rendering helpers
- `registry_cli` itself now stays a thin facade for clap schema plus top-level routing

Rust CLI structure now also includes a `palace_cli` module so the palace-facing
maintenance/read command family no longer lives inline inside `main.rs`:

- `handle_palace_command()` for `compress`, `wake-up`, `recall`, `layers-status`, `migrate`, `repair`, `dedup`, `status`, `doctor`, and `prepare-embedding`
- shared human-readable renderers and JSON error helpers for that command family

Rust palace CLI is now also split by command family instead of keeping every
handler and renderer in one giant file:

- `palace_cli_read` for `compress`, `wake-up`, `recall`, `layers-status`, and `status`
- `palace_cli_maintenance` for `migrate`, `repair`, and `dedup`
- `palace_cli_embedding` for `doctor` and `prepare-embedding`
- `palace_cli_support` for shared config/app/bootstrap helpers used across those handlers

Rust maintenance-facing palace CLI is now also split one step further so
`migrate`, `repair`, and `dedup` do not grow back into one maintenance-sized
file:

- `palace_cli_migrate` for migration command handling plus migrate-specific human/json rendering
- `palace_cli_repair` for `repair`, `repair scan`, `repair prune`, and `repair rebuild`
- `palace_cli_dedup` for duplicate-planning and result rendering
- `palace_cli_maintenance_support` for shared config/app/bootstrap helpers reused by those maintenance handlers
- `palace_cli_maintenance` itself now stays a thin facade that re-exports the maintenance command family

Rust read-facing palace CLI is now also split one step further so
`compress`, `wake-up`, `recall`, `layers-status`, and `status` do not grow back
into one read-sized file:

- `palace_cli_read_compress` for compression command handling plus compress-specific human/json rendering
- `palace_cli_read_layers` for `wake-up`, `recall`, and `layers-status`
- `palace_cli_read_status` for status command handling plus taxonomy-backed human rendering
- `palace_cli_read_support` for shared config/app/bootstrap helpers reused by those read handlers
- `palace_cli_read` itself now stays a thin facade that re-exports the read command family

Rust embedding-facing palace CLI is now also split one step further so
`doctor` and `prepare-embedding` do not keep their renderers and bootstrap
helpers in one file:

- `palace_cli_embedding_doctor` for doctor command handling plus doctor-specific human/json rendering
- `palace_cli_embedding_prepare` for prepare-embedding command handling plus preparation-specific human/json rendering
- `palace_cli_embedding_support` for shared config/app/bootstrap helpers reused by those embedding handlers
- `palace_cli_embedding` itself now stays a thin facade that re-exports the embedding command family

Rust project-facing CLI is now also split by command family instead of keeping
bootstrap, mining, and transcript prep in one file:

- `project_cli_bootstrap` for `init` and `onboarding`
- `project_cli_mining` for `mine` and `search`
- `project_cli_transcript` for `split` and `normalize`
- `project_cli_support` for shared config/app/bootstrap helpers used across those handlers
- `project_cli` itself now stays a thin dispatcher over those project-facing command families

Rust project bootstrap CLI is now also split one step further so `init` and
`onboarding` do not keep their handlers, renderers, and bootstrap helpers in
one file:

- `project_cli_bootstrap_init` for init command handling plus init-specific human/json rendering
- `project_cli_bootstrap_onboarding` for onboarding command handling plus onboarding-specific human/json rendering
- `project_cli_bootstrap_support` for shared app/bootstrap and JSON rendering helpers reused by those bootstrap handlers
- `project_cli_bootstrap` itself now stays a thin facade that re-exports the bootstrap command family

Rust project mining CLI is now also split one step further so `mine` and
`search` do not keep their handlers, renderers, and mining helpers in one file:

- `project_cli_mining_mine` for mine command handling plus mine-specific progress and human/json rendering
- `project_cli_mining_search` for search command handling plus search-specific no-palace and human/json rendering
- `project_cli_mining_support` for shared app/bootstrap and JSON rendering helpers reused by those mining handlers
- `project_cli_mining` itself now stays a thin facade that re-exports the mining command family

Rust project transcript CLI is now also split one step further so `split` and
`normalize` do not keep their handlers and transcript helpers in one file:

- `project_cli_transcript_split` for split command handling
- `project_cli_transcript_normalize` for normalize command handling plus normalize-specific human/json rendering
- `project_cli_transcript_support` for shared JSON rendering helpers reused by those transcript handlers
- `project_cli_transcript` itself now stays a thin facade that re-exports the transcript command family

Rust CLI structure now also includes a `helper_cli` module plus shared `cli_support`
helpers so the remaining control-plane command family no longer lives inline inside
`main.rs`:

- `handle_helper_command()` for `hook`, `instructions`, and `mcp`
- `format_mcp_setup()` / `shell_quote()` plus shared `apply_cli_overrides()`, `palace_exists()`, and `print_no_palace()` for cross-module CLI support

Rust CLI structure now also includes a `root_cli` module so the top-level clap
schema no longer lives inline inside `main.rs`:

- `Cli` and `Command` now live in one schema module shared by the binary entrypoint
- `main.rs` is now reduced to parse + route, instead of also owning every top-level flag definition

Rust CLI structure now also includes a `cli_runtime` module so the top-level
binary route itself no longer lives inline inside `main.rs`:

- `run_cli()` owns the root command dispatch across project, palace, helper, and registry surfaces
- `main.rs` is now effectively just `Cli::parse()` plus one call into the binary runtime

Current first-phase support:

- `init`
- `init` now bootstraps project-local `mempalace.yaml` room config and writes `entities.json` when it can confidently detect people/projects from local prose files
- `init` now also writes palace-ready `aaak_entities.md` and `critical_facts.md` bootstrap docs next to the project when those files do not already exist
- `init` now also writes a project-local `entity_registry.json` bootstrap file seeded from detected people/projects, matching more of Python onboarding's registry surface
- `onboarding <dir>` now provides a dedicated first-run world bootstrap that seeds people/projects/aliases, writes registry + AAAK docs, and can scan local files for missed names before mining
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
- convo normalization now applies Python-style spellcheck to user turns, while preserving known registry names and technical tokens
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
- `recall` to expose Python-style Layer 2 on-demand retrieval by wing/room without semantic search
- `layers-status` to expose Python-style Layer 0-3 stack status in one command
- `hook run --hook session-start|stop|precompact --harness claude-code|codex` for harness-side auto-save integration
- `instructions <help|init|mine|search|status>` to print built-in skill guidance markdown
- `registry summary|lookup|learn` to inspect and update the project-local `entity_registry.json`
- `registry add-person|add-project|add-alias|query` to maintain and query the local entity registry without editing JSON by hand
- `registry research|confirm` to cache Wikipedia-backed research and promote confirmed names into the local registry
- `split <dir>` to detect transcript mega-files, preview session boundaries, and split them into per-session `.txt` files with `.mega_backup` rollover
- `normalize <file>` to inspect how one chat export/transcript is normalized before mining
- `mcp --setup` to print Python-style quick setup instructions for wiring the Rust MCP server into MCP-capable hosts
- `mcp` now defaults to the Python-style quick setup output, while `mcp --serve` explicitly starts the stdio server
- MCP now also includes helper tools for built-in docs and harness hooks: `instructions`, `hook_run`
- MCP now also includes the layer trio: `wake_up`, `recall`, `layers_status`
- MCP now also includes maintenance tools: `repair`, `repair_scan`, `repair_prune`, `repair_rebuild`, `compress`, `dedup`
- MCP now also includes project bootstrap helpers: `onboarding`, `normalize`, `split`
- Rust now has a standalone `registry` surface instead of treating `entity_registry.json` as a bootstrap-only artifact
- `registry lookup` now mirrors Python's ambiguous-name disambiguation for common English words such as `Ever`
- `registry learn` now reuses the local entity detector to append newly discovered people/projects into `entity_registry.json`
- `registry add-alias` now stores nicknames as alias entries with canonical backreferences, so query extraction returns canonical people names
- `registry query` now extracts known people plus still-unknown capitalized candidates from free-form user queries
- `registry research` now populates `wiki_cache` in `entity_registry.json`, following the Python registry path for unknown-name research
- `registry confirm` now promotes cached research into person entries with `source = "wiki"`
- MCP now also includes the project-local registry surface: `registry_summary`, `registry_lookup`, `registry_query`, `registry_learn`, `registry_add_person`, `registry_add_project`, `registry_add_alias`, `registry_research`, `registry_confirm`
- `migrate`
- `repair`
- `repair scan|prune|rebuild` to inspect vector drift, prune queued orphan IDs, and rebuild LanceDB from SQLite
- `dedup` to detect and remove near-identical drawers from the same `source_file`
- `status`
- `doctor`
- `prepare-embedding`
- read-only MCP tools for `status`, `list_wings`, `list_rooms`, `get_taxonomy`, `search`
- read-only MCP tools now also include `check_duplicate` and `get_aaak_spec`, matching more of the Python MCP surface
- read-only MCP tools now also include the room-graph trio: `traverse`, `find_tunnels`, `graph_stats`
- read-only MCP tools now also include the KG read trio: `kg_query`, `kg_timeline`, `kg_stats`
- MCP now also includes the first diary write/read surface: `diary_write`, `diary_read`
- MCP now also includes the first Python-style write surface: `add_drawer`, `delete_drawer`, `kg_add`, `kg_invalidate`
- MCP now also includes registry read/write/research tools against project-local `entity_registry.json`
- MCP write tools now append a palace-local JSONL write-ahead log under `palace/wal/write_log.jsonl`
- provider-based embedding layer with batch document embedding
- embedding internals are now split by concern:
  - `embed_hash` keeps the local hash provider and hashing logic
  - `embed_fastembed` keeps fastembed initialization, warm-up, and doctor integration
  - `embed_runtime_env` keeps ORT / Hugging Face environment and cache helpers
  - `embed` itself stays as the thin public facade over the embedding surface
- SQLite schema version tracking and a minimal migration path
- SQLite storage internals are now also split by concern:
  - `sqlite_schema` keeps schema bootstrap, migrations, and embedding-profile metadata checks
  - `sqlite_drawers` keeps ingested-file state, drawer/compressed-drawer CRUD, and taxonomy/graph readouts
  - `sqlite_kg` keeps knowledge-graph and diary persistence
  - `sqlite` itself now stays as the thin public facade around `SqliteStore`
- LanceDB storage internals are now also split by concern:
  - `vector_schema` keeps table bootstrap, schema definition, and legacy metadata-column upgrades
  - `vector_batch` keeps Arrow batch encoding/decoding for drawer rows
  - `vector_query` keeps add/replace/search/delete flows plus hit decoding and filter SQL helpers
  - `vector` itself now stays as the thin public facade around `VectorStore`
- `migrate` exposes the current SQLite schema upgrade path as a CLI command
- `repair` still provides the old non-destructive diagnostics view by default
- `repair scan` now writes palace-local `corrupt_ids.txt` from SQLite/LanceDB drift and separates `missing_from_vector` from pruneable `orphaned_in_vector`
- `repair prune --confirm` now deletes queued IDs from LanceDB/SQLite using `corrupt_ids.txt`
- `repair rebuild` now re-embeds SQLite drawers and repopulates LanceDB, with a local SQLite backup
- `dedup` now follows Python's source-group workflow: group by `source_file`, keep the richest drawer, and remove near-identical siblings using vector cosine distance
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
- `init` also preserves an existing `entity_registry.json` instead of overwriting it
- `onboarding` refreshes project-local `entities.json`, `entity_registry.json`, `aaak_entities.md`, and `critical_facts.md`, but still keeps an existing `mempalace.yaml` room config instead of overwriting it
- reads `mempalace.yaml` or legacy `mempal.yaml` when present
- room bootstrap and project-mining room routing now share one `room_detector` helper instead of duplicating room vocabulary and fallback rules
- transcript normalization now lives in a standalone `normalize` helper instead of staying embedded inside `convo`
- room-graph traversal and tunnel/stats calculation now live in a standalone `palace_graph` helper instead of staying embedded inside `service`
- KG read/write access now also goes through a standalone `knowledge_graph` facade instead of staying embedded inside `service`
- dedup planning and cosine-distance logic now also live in a standalone `dedup` helper instead of staying embedded inside `service`
- repair diagnostics and scan/prune/rebuild summary assembly now also live in a standalone `repair` helper instead of staying embedded inside `service`
- layer identity loading plus L1/L2 text rendering now also live in the `layers` module instead of staying embedded inside `service`
- search hit basename normalization, similarity rounding, and stable ordering now also live in the `searcher` module instead of staying embedded inside `service`
- manual drawer construction plus SQLite-row-to-input conversion now also live in the `drawers` module instead of staying embedded inside `service`
- AAAK `CompressedDrawer` generation and compression summary assembly now also live in the `compress` module instead of staying embedded inside `service`
- doctor path/version backfill and prepare-embedding warmup retry handling now also live in `embedding_runtime` instead of staying embedded inside `service`
- project/convo mining orchestration plus shared discovery/chunking helpers now also live in `miner` instead of staying embedded inside `service`
- read-side palace summary/search/layer orchestration now also lives in `palace_read` instead of staying embedded inside `service`
- project-local registry load/save/learn/research orchestration now also lives in `registry_runtime` instead of staying embedded inside `service`
- project-local KG/diary/manual-drawer ops now also live in `palace_ops` instead of staying embedded inside `service`
- palace migrate/repair/dedup orchestration now also lives in `maintenance_runtime` instead of staying embedded inside `service`
- palace init/init_project bootstrap orchestration now also lives in `init_runtime` instead of staying embedded inside `service`
- uses config-defined `wing` and `rooms`
- skips init-generated bootstrap artifacts such as `entities.json`, `entity_registry.json`, `aaak_entities.md`, and `critical_facts.md` during normal project mining
- routes files to rooms using path, filename, and keyword scoring
- skips known generated/cache directories and non-readable extensions by default
- supports explicit `--include-ignored` paths for `.gitignore`d files
- project and convo re-mine now share one `palace` helper for `source_mtime`/hash-based unchanged-file checks instead of duplicating that logic in two service branches

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
- `cargo run -- --palace /tmp/mempalace repair scan`
- `cargo run -- --palace /tmp/mempalace repair prune --confirm`
- `cargo run -- --palace /tmp/mempalace repair rebuild`
- `cargo run -- --palace /tmp/mempalace dedup --dry-run`
- `cargo run -- registry summary /path/to/project`
- `cargo run -- registry lookup /path/to/project Riley --context "Riley said the deploy was fixed"`
- `cargo run -- registry learn /path/to/project`
- `cargo run -- registry add-person /path/to/project Riley --relationship daughter --context personal`
- `cargo run -- registry add-project /path/to/project Lantern`
- `cargo run -- registry add-alias /path/to/project Riley Ry`
- `cargo run -- registry query /path/to/project "Ry said Lantern should ship with Max"`
- `cargo run -- registry research /path/to/project Riley --human`
- `cargo run -- registry confirm /path/to/project Riley --type person --relationship daughter --context personal --human`
- `cargo run -- onboarding /path/to/project --mode combo --person "Riley,daughter,personal" --project Lantern --alias Ry=Riley --scan --auto-accept-detected --human`
- `cargo run -- normalize /path/to/transcript.jsonl --human`
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
- `mempalace_wake_up`, `mempalace_recall`, and `mempalace_layers_status` now expose the Rust Layer 0-3 surfaces through MCP
- `mempalace_repair`, `mempalace_repair_scan`, `mempalace_repair_prune`, `mempalace_repair_rebuild`, `mempalace_compress`, and `mempalace_dedup` now expose the Rust maintenance/AAAK surface through MCP
- `mempalace_onboarding`, `mempalace_normalize`, and `mempalace_split` now expose the Rust project bootstrap and transcript-prep surface through MCP
- `mempalace_instructions` and `mempalace_hook_run` now expose the built-in instruction markdown and harness hook runner through MCP
- `mempalace_traverse`, `mempalace_find_tunnels`, and `mempalace_graph_stats` now expose a Python-style room graph built from Rust drawer metadata
- `mempalace_kg_query`, `mempalace_kg_timeline`, and `mempalace_kg_stats` now expose a Python-style temporal KG read surface built from Rust SQLite triples
- `mempalace_kg_add` and `mempalace_kg_invalidate` now expose Python-style KG write operations with structured success payloads
- `mempalace_add_drawer` and `mempalace_delete_drawer` now expose Python-style drawer write/delete operations backed by Rust SQLite + LanceDB
- `mempalace_diary_write` and `mempalace_diary_read` now expose a Python-style agent diary surface backed by Rust SQLite
- write MCP tools now append audit entries before execution to `palace/wal/write_log.jsonl`, keeping Rust's local-first data under the palace root instead of a global home-level WAL path
- empty palaces return the Python-style `{"error":"No palace found","hint":"Run: ..."}` shape
- execution failures in MCP tools now also return tool-level `{"error":"...","hint":"..."}` payloads instead of escalating transport errors
- the newer maintenance/bootstrap MCP tools follow the same tool-level `error + hint` pattern for broken SQLite, missing args, and unsupported transcript inputs
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

- direct compatibility with Python palace data

Current repair scope:

- `repair` checks whether SQLite and LanceDB paths exist
- `repair` reports `schema_version`, embedding profile, and SQLite drawer count
- `repair` checks whether the current LanceDB table is accessible
- `repair scan` compares SQLite drawer IDs with LanceDB drawer IDs and writes palace-local `corrupt_ids.txt`
- `repair prune --confirm` removes queued vector-orphan IDs from both stores
- `repair rebuild` clears and repopulates LanceDB from SQLite using the current embedder profile
