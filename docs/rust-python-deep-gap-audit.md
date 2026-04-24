# Rust/Python Deep Semantic Audit

## Scope

- Target repo: `/Users/dev/workspace2/agents_research/mempalace`
- Reference implementation: `python/`
- Rewrite under audit: `rust/`
- Audit slice: Task 1 seeding plus public interface inventory
- Goal: identify confirmed deep semantic gaps, not speculative concerns

## Audit Method

1. Use Python public entrypoints and tests as the behavioral reference.
2. Compare against Rust public entrypoints, Rust tests, and Rust runtime code.
3. Classify every finding as one of:
   - `confirmed gap`
   - `intentional divergence`
   - `not a gap`
   - `already covered by parity tests`

## Current Repo State

- `docs/parity-ledger.md` already records that Python CLI surface is a subset of Rust CLI surface and Python MCP surface is a subset of Rust MCP surface.
- `docs/parity-ledger.md` also records completed representative behavior audits for transcript split/normalize, layers and maintenance, registry/KG/read-diary, and conversation/read-side behavior.
- This Task 1 slice seeds the durable audit docs, verifies the interface baseline against current source, and avoids reopening family-level semantic work before direct source/test comparison.

## Interface Inventory

### Python CLI public surface

- Root parser and dispatch live in `python/mempalace/cli.py:396-590`.
- Top-level commands currently exported there are `init`, `mine`, `search`, `compress`, `wake-up`, `split`, `hook`, `instructions`, `repair`, `mcp`, `migrate`, and `status`.
- Nested public CLI surfaces are `hook run` and `instructions {init|search|mine|help|status}`.

### Rust CLI public surface

- Root clap command tree lives in `rust/src/root_cli.rs:9-288`.
- Rust includes every Python CLI family above through `Init`, `Mine`, `Search`, `Compress`, `WakeUp`, `Split`, `Hook`, `Instructions`, `Repair`, `Mcp`, `Migrate`, and `Status`.
- Rust also exposes extension surfaces not present in Python: `Onboarding`, `Normalize`, `Recall`, `LayersStatus`, `Dedup`, `Doctor`, `PrepareEmbedding`, and `Registry`.

### Python MCP public surface

- Python MCP transport exposes tools through `handle_request(...): tools/list` in `python/mempalace/mcp_server.py:848-907`.
- Tool handlers currently exported from `python/mempalace/mcp_server.py:139-575` are:
  - Read-side: `mempalace_status`, `mempalace_list_wings`, `mempalace_list_rooms`, `mempalace_get_taxonomy`, `mempalace_search`, `mempalace_check_duplicate`, `mempalace_get_aaak_spec`, `mempalace_traverse`, `mempalace_find_tunnels`, `mempalace_graph_stats`
  - Write/read-write: `mempalace_add_drawer`, `mempalace_delete_drawer`, `mempalace_kg_query`, `mempalace_kg_add`, `mempalace_kg_invalidate`, `mempalace_kg_timeline`, `mempalace_kg_stats`, `mempalace_diary_write`, `mempalace_diary_read`

### Rust MCP public surface

- Rust MCP transport exposes tools through `rust/src/mcp_schema.rs:8-15`, which aggregates read, write, project, and registry catalogs.
- Rust read catalog in `rust/src/mcp_schema_catalog_read.rs:5-159` includes all Python read-side tools and adds `mempalace_wake_up`, `mempalace_recall`, and `mempalace_layers_status`.
- Rust write catalog in `rust/src/mcp_schema_catalog_write.rs:5-130` includes all Python write/read-write tools and adds `mempalace_repair`, `mempalace_repair_scan`, `mempalace_repair_prune`, `mempalace_repair_rebuild`, `mempalace_compress`, and `mempalace_dedup`.
- Rust project catalog in `rust/src/mcp_schema_catalog_project.rs:5-77` adds `mempalace_onboarding`, `mempalace_normalize`, `mempalace_split`, `mempalace_instructions`, and `mempalace_hook_run`.
- Rust registry catalog in `rust/src/mcp_schema_catalog_registry.rs:5-122` adds the `mempalace_registry_*` family.

## Capability Families

### 1. CLI and help/error semantics

| Topic | Python reference | Rust reference | Verdict | Evidence | Notes |
| --- | --- | --- | --- | --- | --- |
| Public CLI surface | `python/mempalace/cli.py:396-590` | `rust/src/root_cli.rs:9-288` | `not a gap` | Python exports 12 top-level CLI commands plus nested `hook run` and `instructions ...`; Rust command tree contains the full Python set and then extends it with Rust-only commands. | This closes only surface presence. Help text, human/json output, and error-shape semantics still need family-level audit below. |
| CLI `mcp --palace ~/...` custom-path handling | `python/tests/test_cli.py::test_mcp_command_uses_custom_palace_path_when_provided`, `python/mempalace/cli.py::cmd_mcp` | `rust/src/config.rs::AppConfig::resolve`, `rust/src/config.rs::normalize_path`, `rust/src/helper_cli.rs`, `rust/src/cli_support.rs` | `confirmed gap` | Python expands `~` before printing MCP setup guidance for a custom palace path; Rust normalizes the path without an `expanduser` step, so quoted or programmatic `~/...` inputs can remain literal or become `<cwd>/~/...`. | This is the clearest CLI-side path-shape mismatch found in this audit family. |
| CLI MCP help/setup wording | `python/tests/test_cli.py::test_mcp_command_prints_setup_guidance` | `rust/src/root_cli.rs`, `rust/tests/cli_integration.rs` | `intentional divergence` | Rust deliberately exposes `mempalace-rs mcp --serve` plus `--setup`/`--serve` workflow instead of mirroring Python’s `python -m mempalace.mcp_server` wording. | Do not list as a gap. |
| Repair trailing-slash recursion class | `python/tests/test_cli.py::test_cmd_repair_trailing_slash_does_not_recurse`, `python/mempalace/cli.py` | `rust/src/repair.rs::backup_sqlite_source` | `not a gap` | Python needed a dedicated guard because it backed up the palace directory path; Rust backs up the concrete SQLite file path beside itself, so the same recursion class does not apply. | Do not list as a gap. |

### 2. MCP tool and payload semantics

| Topic | Python reference | Rust reference | Verdict | Evidence | Notes |
| --- | --- | --- | --- | --- | --- |
| Public MCP surface | `python/mempalace/mcp_server.py:139-575`, `python/mempalace/mcp_server.py:848-907` | `rust/src/mcp_schema.rs:8-15`, `rust/src/mcp_schema_catalog_read.rs:5-159`, `rust/src/mcp_schema_catalog_write.rs:5-130`, `rust/src/mcp_schema_catalog_project.rs:5-77`, `rust/src/mcp_schema_catalog_registry.rs:5-122` | `not a gap` | Every Python MCP tool family is present in Rust catalogs; Rust adds project bootstrap, registry, maintenance, and layer-oriented extension tools beyond the Python tool list. | This closes only public tool presence. Payload shape, coercion, and tool-level error semantics still need deeper family audits. |
| MCP initialize missing `protocolVersion` | `python/tests/test_mcp_server.py::test_initialize_missing_version_uses_oldest`, `python/mempalace/mcp_server.py::handle_request` | `rust/src/mcp_schema_support.rs::negotiate_protocol`, `rust/tests/mcp_integration.rs` | `confirmed gap` | Python falls back to the oldest supported protocol version when the client omits `params.protocolVersion`; Rust currently returns `SUPPORTED_PROTOCOL_VERSIONS[1]`, and Rust integration coverage does not exercise the missing-version case. | This is a concrete transport-level semantic mismatch. |
| MCP no-palace payload shape | `python/mempalace/mcp_server.py::_no_palace` | `rust/src/mcp_schema_support.rs::no_palace`, `rust/tests/mcp_integration.rs::mcp_read_tools_return_python_style_no_palace_response` | `not a gap` | Rust returns the same `error` + `hint` payload shape the Python server uses. | Do not list as a gap. |
| `arguments: null` on zero-arg MCP calls | `python/tests/test_mcp_server.py::test_null_arguments_does_not_hang` | `rust/src/mcp.rs`, `rust/src/mcp_runtime_read.rs` | `not a gap` | Rust accepts non-object `arguments` for zero-arg calls such as `mempalace_status`, matching the Python behavior. | Do not list as a gap. |
| Unknown tool / method transport error shape | `python/tests/test_mcp_server.py` unknown-tool / unknown-method cases | `rust/src/mcp.rs`, `rust/src/mcp_runtime.rs` | `not a gap` | Rust returns the same `-32601` transport-level error shape for unknown tool/method dispatch. | Do not list as a gap. |
| Missing required MCP args | Python tool functions plus generic server exception handling | `rust/tests/mcp_integration.rs` missing-arg coverage across read/write/project/registry/diary tools | `intentional divergence` | Rust deliberately hardens missing-arg handling into tool-level `error` + `hint` payloads instead of mirroring Python’s generic transport-level tool error. | Keep as intentional hardening, not a parity gap. |

### 3. Layers / maintenance semantics

| Topic | Python reference | Rust reference | Verdict | Evidence | Notes |
| --- | --- | --- | --- | --- | --- |
| Layer 0 trim + token estimate | `python/tests/test_layers.py` | `rust/tests/parity_layers_maintenance.rs`, `rust/src/layers.rs` | `already covered by parity tests` | Rust now has explicit focused parity coverage for identity trimming and Python-style char-based token estimation. | Do not list as a gap. |
| Layer1 global-size overflow behavior | `python/tests/test_layers.py::test_layer1_respects_max_chars` | `rust/src/layers.rs::render_layer1` | `confirmed gap` | Python requires a global `MAX_CHARS` cap plus an overflow notice (`more in L3 search`); Rust only truncates per-snippet and limits entries per room. | This is a real user-visible read-side semantic gap. |
| Layer1 importance fallback behavior | `python/tests/test_layers.py::test_layer1_importance_from_various_keys` | `rust/src/layers.rs::render_layer1`, `rust/src/storage/sqlite.rs::DrawerRecord` | `confirmed gap` | Python Layer1 ordering/rendering uses `importance`, `emotional_weight`, `weight`, then a default fallback; Rust `DrawerRecord` has no importance-like field and `render_layer1` does not model this path. | This is a deeper semantic mismatch, not just a missing test. |
| Dedup short / empty docs | `python/tests/test_dedup.py::test_dedup_source_group_short_docs_deleted`, `python/tests/test_dedup.py::test_dedup_source_group_empty_doc_deleted` | `rust/tests/parity_layers_maintenance.rs`, `rust/src/dedup.rs::Deduplicator::plan` | `already covered by parity tests` | Rust now deletes sub-20-char drawers during planning and the focused parity test locks the short-doc path. | Python `None`-document handling is replaced by Rust’s non-nullable `String` model and should not be listed as a gap. |
| Dedup query-failure keep behavior | `python/tests/test_dedup.py::test_dedup_source_group_query_failure_keeps` | `rust/src/maintenance_runtime.rs::dedup`, `rust/src/dedup.rs::Deduplicator::plan` | `intentional divergence` | Python’s rule is specific to per-item Chroma query failures; Rust preloads vector rows once and compares them in memory, so that exact failure mode is removed rather than mirrored. | Do not list as a remaining parity gap. |
| Repair prune preview non-mutation | `python/tests/test_repair.py::test_prune_corrupt_dry_run` | `rust/tests/parity_layers_maintenance.rs`, `rust/src/maintenance_runtime.rs::repair_prune`, `rust/src/repair.rs::read_corrupt_ids` | `already covered by parity tests` | Rust focused parity coverage already verifies queue counting and no-mutation preview behavior. | Do not list as a gap. |
| Repair prune delete-failure fallback and failure accounting | `python/tests/test_repair.py::test_prune_corrupt_delete_failure_fallback` | `rust/src/maintenance_runtime.rs::repair_prune` | `confirmed gap` | Python falls back from batch delete to per-id delete and tracks failures; Rust performs one SQLite delete and one vector delete, then hardcodes `failed: 0`. | This is the clearest remaining repair semantic gap in this family. |
| Repair scan/rebuild drift loop | `python/tests/test_repair.py`, Chroma rebuild path | `rust/tests/service_integration.rs::repair_scan_prune_and_rebuild_handle_vector_drift`, `rust/src/maintenance_runtime.rs::repair_scan`, `repair_rebuild` | `intentional divergence` | Rust explicitly shifts repair from Chroma corruption probing to SQLite/Lance reconciliation and rebuild-from-SQLite. | This matches the accepted repair-model divergence in the parity ledger. |
| Compress summary semantics | Python reference absent in this family’s parity tests | `rust/src/compress.rs`, `rust/src/compression_runtime.rs`, `rust/tests/service_integration.rs::compress_stores_aaak_summaries_and_wake_up_uses_identity` | `not a gap` | No Python parity reference was found for a compress summary contract in the audited files. | Rust-only behavior should not be elevated into a parity gap. |

### 4. Registry / KG / diary semantics

| Topic | Python reference | Rust reference | Verdict | Evidence | Notes |
| --- | --- | --- | --- | --- | --- |
| Registry load default mode | `python/tests/test_entity_registry.py::test_load_from_nonexistent_dir` | `rust/src/registry_io.rs::EntityRegistry::load` | `confirmed gap` | Python loads a missing registry in `personal` mode; Rust falls back to `Self::empty(\"work\")`. | This is a concrete semantic mismatch in empty-state behavior. |
| Registry seed empty-name filtering | `python/tests/test_entity_registry.py::test_seed_skips_empty_names`, `python/mempalace/entity_registry.py::EntityRegistry.seed` | `rust/src/registry_io.rs::EntityRegistry::seed` | `confirmed gap` | Python trims/skips blank names; Rust seeds every `SeedPerson` without an empty-name guard. | This is a correctness and hygiene gap, not just test coverage drift. |
| Registry context disambiguation / alias / query extraction | `python/tests/test_entity_registry.py::test_lookup_alias`, `::test_lookup_ambiguous_word_as_person`, `::test_lookup_ambiguous_word_as_concept` | `rust/tests/parity_registry_kg_ops.rs`, `rust/src/registry_lookup.rs`, `rust/tests/service_integration.rs::registry_summary_lookup_and_learn_work` | `already covered by parity tests` | Rust lookup/disambiguation behavior matches the audited Python paths and is explicitly exercised. | Do not list as a gap. |
| Confirmed wiki-cache lookup behavior | `python/mempalace/entity_registry.py::EntityRegistry.lookup` | `rust/src/registry_lookup.rs::EntityRegistry::lookup` | `confirmed gap` | Python lookup can return confirmed `wiki_cache` entries; Rust lookup checks only `people` and `projects` before returning `unknown`. | Current Rust coverage exercises confirm/research flow, but not lookup-through-cache semantics. |
| KG add auto-create + stats | `python/tests/test_knowledge_graph.py::test_add_triple_creates_entities` | `rust/tests/parity_registry_kg_ops.rs`, `rust/src/knowledge_graph.rs`, `rust/src/storage/sqlite_kg.rs` | `already covered by parity tests` | The representative triple-add and stats path is already locked in Rust. | Do not list as a gap. |
| KG entity CRUD / entity-id semantics | `python/tests/test_knowledge_graph.py::TestEntityOperations`, `python/mempalace/knowledge_graph.py::KnowledgeGraph.add_entity` | `rust/src/knowledge_graph.rs`, `rust/src/storage/sqlite_kg.rs::kg_stats` | `confirmed gap` | Python has explicit entity CRUD with normalized IDs and upsert semantics; Rust exposes only triple/invalidate/query/timeline/stats and derives entity counts from triples. | This is a deeper model gap, not a surface-coverage issue. |
| KG duplicate-active-triple dedup | `python/tests/test_knowledge_graph.py::test_duplicate_triple_returns_existing_id` | `rust/src/storage/sqlite_kg.rs::add_kg_triple` | `confirmed gap` | Python returns the existing active triple ID for a duplicate; Rust always inserts a new row and new ID. | Current Rust parity coverage only checks first-insert behavior. |
| KG query / timeline / stats over triple-backed facts | `python/tests/test_knowledge_graph.py::TestQueries`, `::TestTimeline`, `::TestStats` | `rust/src/storage/sqlite_kg.rs`, `rust/src/knowledge_graph.rs::knowledge_graph_round_trip_and_stats_work` | `not a gap` | As-of filtering, invalidation, timeline limit, and stats behavior align in the audited triple-backed paths. | Keep the duplicate-active-triple row separate as the remaining exception. |
| Diary read/write result semantics | `python/mempalace/mcp_server.py::tool_diary_write`, `::tool_diary_read` | `rust/src/storage/sqlite_kg.rs`, `rust/src/palace_ops.rs`, `rust/tests/parity_registry_kg_ops.rs`, `rust/tests/mcp_integration.rs` | `already covered by parity tests` | User-visible diary read/write semantics already align and are explicitly covered in Rust tests. | Do not list as a gap. |
| Manual add/delete drawer semantics | `python/mempalace/mcp_server.py::tool_add_drawer`, `::tool_delete_drawer` | `rust/src/palace_ops.rs`, `rust/tests/mcp_integration.rs::mcp_add_and_delete_drawer_work`, `rust/tests/service_integration.rs::manual_add_does_not_insert_sqlite_when_embedding_fails` | `not a gap` | The audited happy-path add/delete and failed-add rollback semantics match the exposed Python behavior. | Keep storage-backend differences as intentional divergence rather than parity gaps. |

### 5. Conversation mining / general extractor / read-side semantics

| Topic | Python reference | Rust reference | Verdict | Evidence | Notes |
| --- | --- | --- | --- | --- | --- |
| Conversation re-mine replacement semantics | `python/mempalace/convo_miner.py::mine_convos` purge-before-refile path | `rust/tests/parity_convo_behavior.rs::parity_convo_mining_replaces_existing_source_chunks`, `rust/tests/service_integration.rs::service_mine_convos_exchange_replaces_existing_source_chunks`, `rust/src/miner_convo.rs::mine_conversations_run` | `already covered by parity tests` | Python deletes stale source drawers before refiling modified conversation chunks; Rust representative parity tests explicitly verify modified-source re-mine collapses prior chunks to the new replacement set. | Do not list as a gap. |
| General extractor resolved-problem / positive-emotional classification | `python/tests/test_general_extractor.py`, `python/mempalace/general_extractor.py::_has_resolution`, `::_get_sentiment` | `rust/tests/parity_convo_behavior.rs::parity_general_extractor_keeps_positive_resolved_text_out_of_problem`, `rust/tests/service_integration.rs::service_general_extractor_classifies_decision_preference_milestone_problem_emotional`, `rust/src/convo_general.rs` | `already covered by parity tests` | Python promotes resolved problems and positive text away from raw `problem`; Rust focused parity coverage locks the same representative behavior, including keeping positive resolved text out of `problem`. | Do not list as a gap. |
| Conversation file scan filters | `python/mempalace/convo_miner.py::scan_convos` | `rust/tests/service_integration.rs::service_mine_convos_skips_meta_json_symlink_and_large_files`, `rust/src/convo_scan.rs` | `not a gap` | Python skips `.meta.json`, symlinks, and oversized files; Rust integration coverage exercises the same scan-level exclusions for convos ingestion. | Do not list as a gap. |
| Wake-up / read-side identity preservation | Python wake-up read path plus current representative audit scope | `rust/tests/parity_convo_behavior.rs::parity_wake_up_preserves_identity_and_kind` | `already covered by parity tests` | The representative convo/read-side audit already locks the wake-up identity and kind behavior that this family depends on. | Do not list as a gap. |

### 6. Normalize / split deep edge cases

| Topic | Python reference | Rust reference | Verdict | Evidence | Notes |
| --- | --- | --- | --- | --- | --- |
| Normalize malformed `.json` / `.jsonl` fallback-to-raw behavior | `python/mempalace/normalize.py::normalize`, `::_try_normalize_json`, `python/tests/test_normalize.py::test_try_normalize_json_invalid_json`, `::test_try_normalize_json_valid_but_unknown_schema` | `rust/src/normalize.rs::normalize_conversation` | `confirmed gap` | Python tries JSON normalization and, if schema detection fails or parsing fails, falls back to returning raw content; Rust returns `Ok(None)` for `.json` / `.jsonl` files when JSON normalization does not match, causing those files to be skipped by convo mining instead of ingested as plain text. | This is the one confirmed deep semantic gap found in this family. |
| Normalize invalid UTF-8 tolerance | `python/mempalace/normalize.py::normalize`, `python/tests/test_normalize.py` invalid-byte path | `rust/src/normalize.rs` tests `normalize_file_tolerates_invalid_utf8_like_python`, `rust/tests/cli_integration.rs::cli_normalize_tolerates_invalid_utf8_like_python` | `not a gap` | Both implementations read with replacement semantics and preserve surrounding plain transcript content. | Do not list as a gap. |
| Normalize large-file guard | `python/tests/test_normalize.py::test_normalize_rejects_large_file`, `python/mempalace/normalize.py::normalize` | `rust/src/normalize.rs` tests `normalize_file_rejects_files_over_python_size_limit` | `not a gap` | Both sides reject files over the same 500 MB ceiling before full read/normalization. | Do not list as a gap. |
| Existing-quote transcript pass-through | `python/tests/test_normalize.py::test_normalize_already_has_markers`, `python/mempalace/normalize.py::normalize` | `rust/src/normalize.rs` tests `normalize_existing_quote_transcript_passes_through_like_python`, `normalize_quote_markers_without_space_count_like_python` | `already covered by parity tests` | Rust explicitly locks Python-style quote-marker pass-through, including the marker-count edge case. | Do not list as a gap. |
| Split mega-file boundary / tiny-fragment / backup behavior | `python/tests/test_split_mega_files.py`, `python/mempalace/split_mega_files.py` | `rust/src/split.rs`, `rust/tests/cli_integration.rs::cli_split_dry_run_reports_output_without_writing`, `::cli_split_writes_files_and_renames_backup`, `::cli_split_file_mode_limits_scan_to_requested_file` | `not a gap` | Rust mirrors Python’s true-session boundary detection, skips tiny fragments, preserves dry-run semantics, and renames the source to `.mega_backup` when writing outputs. | No confirmed split-family semantic gap surfaced in this audit. |

## Summary

- Confirmed gaps:
  - Layer1 global-size overflow behavior is missing in Rust.
  - Layer1 importance fallback behavior is missing in Rust.
  - `repair_prune` delete-failure fallback and failure accounting are missing in Rust.
  - Registry load default mode differs on missing-registry empty state.
  - Registry seed empty-name filtering is missing in Rust.
  - Registry lookup does not return confirmed wiki-cache entries.
  - KG entity CRUD / entity-id semantics are missing in Rust.
  - KG duplicate-active-triple dedup is missing in Rust.
  - MCP initialize without `protocolVersion` falls back differently in Rust.
  - CLI `mcp --palace ~/...` custom-path handling differs because Rust does not expand `~` like Python does.
  - Malformed `.json` / `.jsonl` normalization falls back to raw content in Python but is skipped in Rust.
- Intentional divergences: none newly recorded in Task 1; keep using `docs/parity-ledger.md` as the current source for accepted divergences
- Already-covered representative audits: not reopened in Task 1
- Not-a-gap closures:
  - Public CLI surface
  - Public MCP surface
