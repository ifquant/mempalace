# Rust/Python Deep Semantic Gap List

This file lists only **confirmed remaining gaps** between the current Python MemPalace behavior and the Rust rewrite.

## Rules

- Do not list intentional divergences.
- Do not list Rust supersets.
- Do not list already-covered representative audits.
- Every row must point to concrete Python and Rust evidence.

## Confirmed Gaps

None currently.

The residual parity batches recorded by the deep audit are now closed for:

- Layer1 global-cap and importance semantics
- `repair_prune` delete-failure fallback and failure accounting
- registry empty-state, seed filtering, and confirmed wiki-cache lookup behavior
- KG entity CRUD / normalized entity-id semantics and duplicate-active-triple reuse
- MCP initialize protocol fallback
- CLI `mcp --palace ~/...` path expansion
- normalize raw fallback for unknown or malformed `.json` / `.jsonl`

If a new user-visible gap is found later, record it here first and then mirror the summary in `docs/parity-ledger.md`.

## Closed During Audit

| Topic | Resolution |
| --- | --- |
| Layer1 global-size overflow behavior | Closed by residual parity implementation. Rust `render_layer1()` now enforces the Python-style global character cap and emits the same overflow-hint class, with focused parity coverage in `rust/tests/parity_layers_maintenance.rs`. |
| Layer1 importance fallback behavior | Closed by residual parity implementation. Rust now persists canonical drawer `importance`, sorts Layer1 entries with Python-style fallback semantics, and locks the ordering in focused parity tests. |
| `repair_prune` delete-failure fallback and failure accounting | Closed by residual parity implementation. Rust now falls back from batch delete to per-ID deletion and reports real failure counts like the Python path. |
| Registry load default mode | Closed by residual parity implementation. Rust missing-registry load now defaults to `personal`, matching Python empty-state behavior. |
| Registry seed empty-name filtering | Closed by residual parity implementation. Rust now trims and skips blank seed names before persisting registry data. |
| Confirmed wiki-cache lookup behavior | Closed by residual parity implementation. Rust lookup now surfaces confirmed `wiki_cache` entries before returning `unknown`, with focused parity coverage. |
| KG entity CRUD / entity-id semantics | Closed by residual parity implementation. Rust now has explicit entity upsert behavior with normalized IDs backed by durable SQLite entity storage. |
| KG duplicate-active-triple dedup | Closed by residual parity implementation. Rust now reuses the active triple ID for duplicate inserts instead of creating a fresh fact row. |
| MCP initialize missing `protocolVersion` | Closed by residual parity implementation. Rust now falls back to the oldest supported protocol version when `protocolVersion` is omitted. |
| CLI `mcp --palace ~/...` custom-path handling | Closed by residual parity implementation. Rust now expands `~` before rendering MCP setup output, matching Python path-shape behavior. |
| Normalize malformed `.json` / `.jsonl` fallback-to-raw behavior | Closed by residual parity implementation. Rust now falls back to raw content for unknown-schema JSON and malformed JSONL instead of skipping the file. |
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
