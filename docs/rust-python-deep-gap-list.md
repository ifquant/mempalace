# Rust/Python Deep Semantic Gap List

This file lists only **confirmed remaining gaps** between the current Python MemPalace behavior and the Rust rewrite.

## Rules

- Do not list intentional divergences.
- Do not list Rust supersets.
- Do not list already-covered representative audits.
- Every row must point to concrete Python and Rust evidence.

## Confirmed Gaps

| Family | Gap | Python reference | Rust reference | Severity | Suggested batch |
| --- | --- | --- | --- | --- | --- |
| Layers / maintenance | Layer1 global-size overflow behavior is missing: Rust does not enforce Python’s `MAX_CHARS`-style global cap or emit the overflow notice. | `python/tests/test_layers.py::test_layer1_respects_max_chars` | `rust/src/layers.rs::render_layer1` | high | `read-side residual parity` |
| Layers / maintenance | Layer1 importance fallback behavior is missing: Rust has no equivalent to Python’s `importance` / `emotional_weight` / `weight` fallback semantics. | `python/tests/test_layers.py::test_layer1_importance_from_various_keys` | `rust/src/layers.rs::render_layer1`, `rust/src/storage/sqlite.rs::DrawerRecord` | high | `read-side residual parity` |
| Layers / maintenance | `repair_prune` lacks Python’s delete-failure fallback and failure accounting. | `python/tests/test_repair.py::test_prune_corrupt_delete_failure_fallback` | `rust/src/maintenance_runtime.rs::repair_prune` | high | `maintenance residual parity` |
| Registry / KG | Missing registry load default parity: Rust loads a missing registry in `work` mode, while Python defaults to `personal`. | `python/tests/test_entity_registry.py::test_load_from_nonexistent_dir` | `rust/src/registry_io.rs::EntityRegistry::load` | medium | `registry residual parity` |
| Registry / KG | Missing registry seed empty-name filtering: Rust seeds blank names that Python drops. | `python/tests/test_entity_registry.py::test_seed_skips_empty_names` | `rust/src/registry_io.rs::EntityRegistry::seed` | medium | `registry residual parity` |
| Registry / KG | Missing confirmed wiki-cache lookup path: Rust lookup does not surface confirmed wiki-cache entries the way Python does. | `python/mempalace/entity_registry.py::EntityRegistry.lookup` | `rust/src/registry_lookup.rs::EntityRegistry::lookup` | medium | `registry residual parity` |
| Registry / KG | Missing KG entity CRUD / normalized entity-id semantics. | `python/tests/test_knowledge_graph.py::TestEntityOperations`, `python/mempalace/knowledge_graph.py::KnowledgeGraph.add_entity` | `rust/src/knowledge_graph.rs`, `rust/src/storage/sqlite_kg.rs` | high | `knowledge-graph residual parity` |
| Registry / KG | Missing duplicate-active-triple dedup semantics: Rust inserts a fresh triple instead of returning the active existing triple ID. | `python/tests/test_knowledge_graph.py::test_duplicate_triple_returns_existing_id` | `rust/src/storage/sqlite_kg.rs::add_kg_triple` | high | `knowledge-graph residual parity` |
| CLI / MCP | Missing-protocol MCP initialize fallback differs: Rust does not fall back to the oldest supported protocol version when `params.protocolVersion` is omitted. | `python/tests/test_mcp_server.py::test_initialize_missing_version_uses_oldest` | `rust/src/mcp_schema_support.rs::negotiate_protocol` | medium | `mcp residual parity` |
| CLI / MCP | Custom `mcp --palace ~/...` handling differs because Rust does not expand `~` the way Python does before emitting setup guidance. | `python/tests/test_cli.py::test_mcp_command_uses_custom_palace_path_when_provided` | `rust/src/config.rs::AppConfig::resolve`, `rust/src/helper_cli.rs`, `rust/src/cli_support.rs` | medium | `cli path-shape residual parity` |
| Normalize / split | Malformed `.json` / `.jsonl` normalize fallback differs: Python falls back to raw content when JSON normalization fails, while Rust returns `None` and skips the file. | `python/mempalace/normalize.py::normalize`, `python/tests/test_normalize.py::test_try_normalize_json_invalid_json`, `python/tests/test_normalize.py::test_try_normalize_json_valid_but_unknown_schema` | `rust/src/normalize.rs::normalize_conversation` | medium | `normalize residual parity` |

## Closed During Audit

| Topic | Resolution |
| --- | --- |
| Public CLI surface | Closed as `not a gap`. `python/mempalace/cli.py:396-590` exports a command tree that is fully represented within `rust/src/root_cli.rs:9-288`, while Rust-only commands remain extension surface rather than missing parity work. |
| Public MCP surface | Closed as `not a gap`. Python `tools/list` and handler inventory in `python/mempalace/mcp_server.py:139-575` and `python/mempalace/mcp_server.py:848-907` are fully represented by Rust `mcp_schema` catalogs in `rust/src/mcp_schema.rs:8-15`, `rust/src/mcp_schema_catalog_read.rs:5-159`, `rust/src/mcp_schema_catalog_write.rs:5-130`, `rust/src/mcp_schema_catalog_project.rs:5-77`, and `rust/src/mcp_schema_catalog_registry.rs:5-122`. |
| Dedup short / empty doc handling | Closed as `already covered by parity tests`. Rust `parity_layers_maintenance.rs` plus `Deduplicator::plan` already lock the representative behavior. |
| Dedup query-failure keep behavior | Closed as `intentional divergence`, not a gap. Python’s per-query Chroma failure mode does not exist in Rust’s preload-and-compare dedup path. |
| Repair prune preview non-mutation | Closed as `already covered by parity tests`. Rust focused parity coverage already verifies queued-ID preview without storage mutation. |
| Compress summary semantics | Closed as `not a gap` in this audit slice because no Python parity contract was found in the reference tests audited here. |
| Registry disambiguation / alias / query extraction | Closed as `already covered by parity tests`. The audited Rust lookup and service tests match the Python reference behavior. |
| KG query / timeline / stats over triple-backed facts | Closed as `not a gap`. The audited triple-backed query/timeline/stats semantics align; the remaining KG exception is duplicate-active-triple handling. |
| Diary read/write result semantics | Closed as `already covered by parity tests`. Rust parity and MCP integration tests already lock the exposed Python behavior. |
| Manual add/delete drawer semantics | Closed as `not a gap` in the audited happy-path and rollback semantics. Storage-backend differences remain intentional divergence, not remaining parity work. |
| Conversation re-mine replacement semantics | Closed as `already covered by parity tests`. Rust `parity_convo_behavior.rs` and `service_integration.rs::service_mine_convos_exchange_replaces_existing_source_chunks` already lock the modified-source replacement behavior Python relies on. |
| General extractor resolved-problem and positive-emotional classification | Closed as `already covered by parity tests`. Rust `parity_convo_behavior.rs` and service-level extractor tests already lock the representative Python classification behavior. |
| Conversation scan filters (`.meta.json`, symlink, oversized file) | Closed as `not a gap`. Rust `service_mine_convos_skips_meta_json_symlink_and_large_files` exercises the same exclusion class as Python `scan_convos`. |
| Normalize invalid UTF-8 tolerance | Closed as `not a gap`. Rust `normalize_file_tolerates_invalid_utf8_like_python` and CLI coverage match Python’s replacement-read behavior. |
| Normalize large-file guard | Closed as `not a gap`. Rust enforces the same 500 MB rejection ceiling as Python and has direct test coverage. |
| Existing-quote transcript pass-through | Closed as `already covered by parity tests`. Rust `normalize.rs` tests explicitly lock Python-style quote-marker pass-through. |
| Split mega-file boundary / tiny-fragment / backup behavior | Closed as `not a gap`. Rust `split.rs` plus CLI integration coverage match the audited Python split semantics in this family. |
| MCP no-palace payload shape | Closed as `not a gap`. Rust returns the same `error` + `hint` payload shape as Python. |
| `arguments: null` for zero-arg MCP calls | Closed as `not a gap`. Rust accepts the same effective call shape Python tolerates. |
| Unknown tool / method transport errors | Closed as `not a gap`. Rust mirrors the `-32601` transport-level behavior. |
| Missing required MCP args | Closed as `intentional divergence`. Rust intentionally hardens these into tool-level `error` + `hint` payloads rather than mirroring Python’s generic transport-level tool error path. |
| CLI MCP setup wording | Closed as `intentional divergence`. Rust intentionally uses its own `mempalace-rs mcp --serve` flow. |
| Repair trailing-slash recursion class | Closed as `not a gap`. Rust backs up the SQLite file directly, so Python’s directory-recursion bug class does not apply. |
