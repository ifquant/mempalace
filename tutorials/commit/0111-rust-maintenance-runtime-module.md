# 背景

随着前几轮不断把 `service.rs` 拆薄，读面、挖掘、registry、palace ops 都已经逐步独立出来。但 maintenance 这一块还集中堆在 `service.rs`：

- `migrate`
- `repair`
- `repair_scan`
- `repair_prune`
- `repair_rebuild`
- `dedup`

这些方法本质上都属于同一类问题：围绕 palace 数据库和 LanceDB 做运维、诊断、迁移和清理。继续散落在 `service.rs` 里，会让 runtime 边界不清楚。

# 主要目标

把 Rust 的 maintenance flow 收成独立 `maintenance_runtime` 模块，让 `service.rs` 只保留更薄的入口层。

# 改动概览

- 新增 [rust/src/maintenance_runtime.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/maintenance_runtime.rs)
- 提供 `MaintenanceRuntime`
- 收进去的能力包括：
  - `migrate()`
  - `repair()`
  - `repair_scan()`
  - `repair_prune()`
  - `repair_rebuild()`
  - `dedup()`
- `service.rs` 对应 maintenance 入口现在统一委托给 `MaintenanceRuntime`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 为什么 `migrate` 也放进 maintenance

`migrate` 虽然表面简单，但它和 `repair / dedup` 一样，都属于：

- palace 生命周期维护
- schema / storage 版本推进
- 运维型 CLI / MCP 表面

所以从职责上看，把它放进同一个 maintenance runtime 是合理的。

## 为什么 `dedup` 不是继续留在 `dedup.rs`

`dedup.rs` 现在负责的是“重复内容判定与 plan 生成”。  
但真正的运行时流程还包括：

- 打开 SQLite
- 打开 LanceDB
- 拉取 drawers
- dry-run / stats-only 分流
- 真删时同步两边
- 最终 summary assembly

这些都是 runtime/orchestration，不属于纯 `dedup` 算法模块，所以收进 `maintenance_runtime` 更清楚。

## 为什么 `repair_rebuild` 仍然在这里直接做 re-embed loop

`repair_rebuild` 是少数必须同时碰：

- SQLite drawers 作为 source of truth
- embedder
- LanceDB 重建

这段逻辑本来就不是底层 storage 层能解决的，它就是 maintenance runtime 的责任。这里显式保留在 `maintenance_runtime`，比继续挂在 `service` 更符合边界。

# 补充知识

模块化并不是“每个主题一个文件”就结束了。真正有效的拆分通常是：

- 纯逻辑模块
- runtime/facade 模块
- 应用入口层

这轮的重点是把运维型 facade 也独立出来，让 `service.rs` 不再同时扮演 CLI 入口和实际 maintenance executor。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有改变 CLI / MCP maintenance 表面
- 这次没有改变 migrate / repair / dedup 的行为语义
- `service.rs` 后续仍然可以继续收口 init/bootstrap 一段
