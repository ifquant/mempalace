# Rust/Python Parity Ledger

This document tracks **user-visible parity** between the current Python MemPalace implementation and the Rust rewrite.

It is intentionally scoped to:

- public CLI surface
- public MCP tool surface
- explicit intentional divergences
- confirmed remaining follow-up work

It is **not** a tracker for internal module splits or general refactor progress.

## Snapshot

- Python CLI public surface is currently a subset of the Rust CLI surface.
- Python MCP public tool surface is currently a subset of the Rust MCP surface.
- Rust adds several local-first extension surfaces that Python does not currently expose.
- Rust CLI help coverage is now broadly locked by integration tests across root/global flags, helper/read flows, maintenance, registry, and project-facing commands.
- Rust README verification examples now cover the main project, palace, registry, helper, MCP, and embedding flows.
- Rust transcript split behavior has completed a focused Python parity pass covering CLI entrypoints, source defaults, lossy transcript reads, output naming, people detection, and `known_names.json`.
- Direct compatibility with existing Python palace data is **not** part of the current Rust phase.

## Python CLI Surface

| Python surface | Rust status | Verdict | Notes |
| --- | --- | --- | --- |
| `init` | Present | `aligned` | Rust also supports human/json output and richer bootstrap summaries. |
| `mine` | Present | `aligned` | Rust keeps `projects` and `convos` modes plus `exchange/general` convo extraction. |
| `search` | Present | `aligned` | Rust also has structured JSON and human renderers. |
| `compress` | Present | `aligned` | Rust persists compressed drawers in SQLite. |
| `wake-up` | Present | `aligned` | Rust keeps Layer 0 + Layer 1 wake-up context. |
| `split` | Present | `aligned` | Rust follows the Python mega-file split flow, including `--source`, `--file`, `MEMPALACE_SOURCE_DIR` defaulting, lossy text reads, Python-style output naming, fallback people detection, and `~/.mempalace/known_names.json`. |
| `hook run` | Present | `aligned` | Rust supports `session-start`, `stop`, and `precompact`. |
| `instructions` | Present | `aligned` | Rust ships the same built-in instruction names. |
| `repair` | Present | `rust superset` | Rust keeps diagnostics and adds `scan`, `prune`, and `rebuild`. |
| `mcp` | Present | `rust superset` | Rust supports setup output and explicit `--serve`. |
| `migrate` | Present | `rust superset` | Rust exposes structured human/json summaries. |
| `status` | Present | `rust superset` | Rust exposes structured output and more detailed local-first status fields. |

## Rust-Only CLI Surface

These are not parity gaps. They are intentional Rust extension surfaces.

| Rust surface | Verdict | Notes |
| --- | --- | --- |
| `onboarding` | `rust superset` | Dedicated first-run bootstrap for project-local entities, registry, and AAAK docs. |
| `normalize` | `rust superset` | Inspect one transcript/chat export normalization before mining. |
| `recall` | `rust superset` | Layer 2 recall without semantic search. |
| `layers-status` | `rust superset` | Layer 0-3 stack status in one command. |
| `dedup` | `rust superset` | Explicit near-duplicate cleanup workflow. |
| `doctor` | `rust superset` | Embedding runtime health inspection. |
| `prepare-embedding` | `rust superset` | Local model/runtime warm-up path. |
| `registry ...` | `rust superset` | Project-local entity registry read/write/research surface. |

## Python MCP Surface

| Python MCP tool | Rust status | Verdict | Notes |
| --- | --- | --- | --- |
| `mempalace_status` | Present | `aligned` | Rust returns Python-style no-palace payloads and local-first status details. |
| `mempalace_list_wings` | Present | `aligned` |  |
| `mempalace_list_rooms` | Present | `aligned` |  |
| `mempalace_get_taxonomy` | Present | `aligned` |  |
| `mempalace_get_aaak_spec` | Present | `aligned` |  |
| `mempalace_search` | Present | `aligned` |  |
| `mempalace_check_duplicate` | Present | `aligned` | Rust returns Python-style duplicate payload shape. |
| `mempalace_traverse` | Present | `aligned` |  |
| `mempalace_find_tunnels` | Present | `aligned` |  |
| `mempalace_graph_stats` | Present | `aligned` |  |
| `mempalace_add_drawer` | Present | `aligned` | Rust also writes palace-local WAL entries. |
| `mempalace_delete_drawer` | Present | `aligned` |  |
| `mempalace_kg_query` | Present | `aligned` |  |
| `mempalace_kg_add` | Present | `aligned` |  |
| `mempalace_kg_invalidate` | Present | `aligned` |  |
| `mempalace_kg_timeline` | Present | `aligned` |  |
| `mempalace_kg_stats` | Present | `aligned` |  |
| `mempalace_diary_write` | Present | `aligned` |  |
| `mempalace_diary_read` | Present | `aligned` |  |

## Rust-Only MCP Surface

These are not parity gaps. They are intentional Rust extension surfaces.

| Rust MCP tool family | Verdict | Notes |
| --- | --- | --- |
| `mempalace_wake_up`, `mempalace_recall`, `mempalace_layers_status` | `rust superset` | Layer-oriented read surfaces not exposed by the current Python MCP server. |
| `mempalace_repair*`, `mempalace_compress`, `mempalace_dedup` | `rust superset` | Maintenance and AAAK operations exposed directly through MCP. |
| `mempalace_onboarding`, `mempalace_normalize`, `mempalace_split` | `rust superset` | Project bootstrap and transcript-prep tools. |
| `mempalace_instructions`, `mempalace_hook_run` | `rust superset` | Helper/doc tooling surfaced directly through MCP. |
| `mempalace_registry_*` | `rust superset` | Project-local registry read/write/research surface. |

## Intentional Divergence

| Topic | Verdict | Notes |
| --- | --- | --- |
| Python palace data compatibility | `intentional divergence` | Rust does not directly read/write the existing Python palace layout in the current phase. |
| On-disk locality | `intentional divergence` | Rust keeps WAL, hook state, and related state under the active palace root instead of Python's home-level global paths. |
| Repair model | `intentional divergence` | Rust keeps Python's repair spirit but extends it into explicit `scan`, `prune`, and `rebuild` subflows. |

## Completed Behavior Audits

These behavior-level parity passes have already been completed and should not be re-opened as generic remaining work without a new concrete gap.

| Area | Verdict | Notes |
| --- | --- | --- |
| Transcript mega-file split | `aligned` | Rust now matches Python for true session boundary filtering, minimum session scanning, `--source`, `--file`, default source directory, dry-run/non-dry-run write behavior, `.mega_backup` rename behavior, lossy UTF-8 reads, timestamp fallback, source stem cleanup/truncation, subject cleanup/truncation, final filename sanitizing, fallback known people, list/object `known_names.json`, and `username_map`. |

## Remaining Work

The following are confirmed follow-up areas, not confirmed missing headline features:

| Area | Verdict | Notes |
| --- | --- | --- |
| Deeper non-CLI behavior audit | `remaining` | Continue checking behavior-level parity beyond command and MCP presence, especially edge-case semantics outside the completed transcript split pass. |
| Future residual parity batches | `remaining` | If a real user-visible gap is found later, add it here first before implementing it. |
