# 背景

`rust/src/service.rs` 之前已经把 `repair`、`dedup`、`compress`、`embedding_runtime` 等块陆续抽成独立模块，但 `mine_project()` 这一段仍然很重：它同时负责项目扫描、chunking、conversation mining、dry-run/progress 事件、SQLite/LanceDB 写入前的 orchestration。这样继续堆在 `service.rs` 里，会让后续对齐 Python `miner.py` 时越来越难维护。

# 主要目标

把 Rust 的 project/conversation mining orchestration 整块抽成独立 `miner` 模块，同时保持现有 CLI / MCP / service 表面不变。

# 改动概览

- 新增 [rust/src/miner.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/miner.rs)
- 把下面这些逻辑从 `service.rs` 迁到 `miner.rs`：
  - `mine_project_run()`
  - `mine_conversations_run()`
  - `discover_files()`
  - `chunk_text()`
  - conversation drawer 构建
  - project/convo 共用的 skip/include helper
- `service.mine_project_with_progress()` 现在只保留：
  - `self.init().await?`
  - 调用 `mine_project_run(...)`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs) 导出 `miner`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md) 记录新的库层边界

# 关键知识

## 为什么这次是抽 `miner`，不是继续拆 `service`

`mine` 是 Rust 对齐 Python 的核心主链路之一。这里同时覆盖：

- project ingestion
- convo ingestion
- room routing
- dry-run / progress
- re-mine skip 语义

如果只抽一两个 helper，`service.rs` 仍然会保留完整的“挖掘状态机”。这次直接抽成 `miner`，后面再继续对齐 Python `miner.py` 时，改动面就会更集中。

## 为什么 `service` 仍然保留 `self.init().await?`

这里刻意没把 palace bootstrap 一并塞进 `miner`。原因是 `init` 仍然是 `App` 的生命周期责任，`miner` 只负责“已具备 palace runtime 前提下如何挖掘”。这样模块边界更清楚：

- `service`: 应用级 orchestration
- `miner`: mining orchestration
- `storage/*`: 持久化

## 为什么 `chunk_text()` 仍然保留 crate-private 可见性

`chunk_text()` 现在主要服务 project mining，但测试里也要直接验证它的分块行为。这里用 `pub(crate)` 而不是 `pub`，是为了：

- 允许 crate 内测试直接复用
- 不把它提前暴露成对外稳定 API

# 补充知识

Python 版 `miner.py` 的一个典型问题是：功能越来越全时，很容易演变成“主流程 + 一堆 helper 全塞同一文件”。Rust 这里的模块化收口，本质上是在用更明确的职责边界，把 Python 已经验证过的行为保留下来，但避免把 Rust 也做成同样难拆的大文件。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有改变 CLI / MCP 表面
- 这次没有再改 project/convo mining 的行为语义，只是迁移 orchestration
- `service.rs` 仍然偏大，后续还可以继续抽 `status/taxonomy` 或 registry 相邻块
