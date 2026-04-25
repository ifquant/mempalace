# 背景

这次任务不是补功能差距，而是做一轮面向审计的 Rust 注释整理。目标文件集中在 `rust/src/` 的 palace、maintenance、SQLite、LanceDB 这些边界层：它们本身行为已经存在，但阅读门槛偏高，尤其是 schema migration、跨库回滚、preview/live 分支这些逻辑，如果没有注释，reviewer 很容易得靠逆向调用链才能确认真实语义。

同时，这一轮有明确约束：

- 只能改指定 write set
- 只做 comment pass，不做预期行为改动
- 不能碰 `python/uv.lock`
- 不能碰 `docs/superpowers/` 下的计划文件

# 主要目标

给 Rust 存储与维护相关模块补上足够稀疏、但对审计真正有帮助的文档注释，让 reviewer 能快速回答下面这些问题：

- palace 路径、SQLite、LanceDB 各自负责什么
- schema 升级和历史数据 backfill 是怎么处理的
- `repair_prune`、`repair_rebuild`、`dedup`、`compress` 的 preview/live 语义分别在哪里
- 跨 SQLite/LanceDB 的写入或删除失败时，代码怎样尽量保持两边一致

# 改动概览

- 给主要子系统文件加了模块级 `//!` 注释
  - `config.rs`
  - `palace.rs`
  - `palace_read.rs`
  - `palace_ops.rs`
  - `compress.rs`
  - `compression_runtime.rs`
  - `dedup.rs`
  - `repair.rs`
  - `maintenance_runtime.rs`
  - `storage/sqlite.rs`
  - `storage/sqlite_drawers.rs`
  - `storage/sqlite_kg.rs`
  - `storage/sqlite_schema.rs`
  - `storage/vector.rs`
  - `storage/vector_batch.rs`
  - `storage/vector_query.rs`
  - `storage/vector_schema.rs`
- 给关键 public struct / function / constant 补了 `///` 注释
  - 重点覆盖 `AppConfig`、`PalaceReadRuntime`、`PalaceOpsRuntime`、`MaintenanceRuntime`、`SqliteStore`、`VectorStore` 以及 repair/dedup/compress 相关审计锚点
- 只在真正容易误读的边界上补了少量 inline comment
  - SQLite 作为 canonical store 时的跨库回滚
  - `repair_scan -> repair_prune` 的 staged file 语义
  - `repair_prune` 批量删除失败后的逐条 fallback
  - `repair_rebuild` 的 backup + clear-then-repopulate 语义
  - SQLite schema migration 中的历史字段 backfill
  - LanceDB metadata 列缺失时的 lazy backfill

# 关键知识

这一轮最重要的阅读框架是区分“canonical store”和“secondary index”：

- `SQLite` 是 canonical store
  - 它保存 drawer 元数据、schema version、KG、maintenance ledger
  - schema migration 也只在这一侧发生
- `LanceDB` 是 secondary index
  - 它负责 semantic search 和一部分 maintenance scan 所需的向量视图
  - 但它不是最终真相来源

这会直接影响代码注释该写在哪：

- 如果是“为什么这里要回滚/恢复”，多半在跨 SQLite/LanceDB 的 runtime 里解释
- 如果是“为什么这里要 backfill 或升级旧表”，多半在 `sqlite_schema.rs` / `vector_schema.rs`
- 如果是“preview 和 live 有什么差别”，多半在 maintenance/compression runtime

# 补充知识

1. Rust 的模块级 `//!` 注释更适合回答“这个文件在系统里的职责是什么”，而 `///` 更适合回答“这个 public item 为何存在、应该怎么看”。做审计文档 pass 时，先补 `//!`，再补少量 `///`，通常比直接到处写 inline comment 更稳。

2. 对于双存储系统，注释不要只解释“做了什么”，而要解释“哪边是 canonical”。否则 reviewer 即使看到了回滚代码，也很难判断为什么失败时优先恢复某一边。

# 验证

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test --test parity_layers_maintenance --quiet
cargo test --test service_integration repair_prune_live_deletes_existing_ids_and_keeps_failure_count_zero --quiet
```

# 未覆盖项

- 没有修改 `python/` 实现，也没有触碰 `python/uv.lock`
- 没有修改 `docs/superpowers/` 下的任何计划或执行文档
- 没有给 mining / normalize / registry / onboarding 这些不在 Task 2 write set 内的文件补注释
