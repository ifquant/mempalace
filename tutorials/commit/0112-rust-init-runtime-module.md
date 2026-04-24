# 背景

经过前几轮收口之后，`service.rs` 里读面、写面、maintenance、registry、miner 都已经逐步拆到独立 runtime。  
但 `init` 和 `init_project` 这两条 palace bootstrap 路径还留在 `service.rs` 里，负责：

- `ensure_dirs`
- SQLite schema init
- embedding profile 校验
- LanceDB/table bootstrap
- project bootstrap 文件写入
- `InitSummary` 组装

这已经是一个完整的初始化 runtime，而不只是一个零散 helper。

# 主要目标

把 Rust 的 palace bootstrap 路径抽成独立 `init_runtime` 模块，让 `service.rs` 继续收薄。

# 改动概览

- 新增 [rust/src/init_runtime.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/init_runtime.rs)
- 提供 `InitRuntime`
- 收进去的能力包括：
  - `prepare_storage()`
  - `init()`
  - `init_project()`
- `prepare_storage()` 统一封装：
  - `ensure_dirs`
  - SQLite open/init
  - embedding profile 校验
  - vector store/table bootstrap
  - schema version 读取
- `service.rs` 里的 `init` / `init_project` 现在都改为 thin wrapper
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 为什么 `init` 和 `init_project` 要放同一个 runtime

这两条路径只有最后一步不同：

- `init()`：只初始化 palace 本体
- `init_project()`：在 palace 初始化后，再对 project 目录做 bootstrap

它们共享同一套 storage bootstrap 逻辑，所以放进一个 runtime 更合理，不应该各自复制。

## 为什么 `prepare_storage()` 返回 `schema_version`

`InitSummary` 最后都要带 `schema_version`。  
如果 `prepare_storage()` 只做 side effects，不返回版本信息，那么 `init` 和 `init_project` 还得各自再查一次 SQLite。这里直接把“准备完成后的 schema version”作为返回值，更紧凑也更不容易漏。

## 为什么 project bootstrap 仍然留给 `bootstrap_project()`

这次没有把 bootstrap 细节重新搬进 `init_runtime`。职责边界保持为：

- `bootstrap.rs`: project bootstrap 规则和文件生成
- `init_runtime.rs`: init/init_project orchestration

这样不会把“初始化 storage”和“如何写项目世界文件”混成一个模块。

# 补充知识

很多代码库在重构时会只抽“复杂逻辑”，忽略这种看似重复但很关键的 bootstrap 路径。  
实际上，初始化链路往往决定了：

- CLI 首次使用体验
- no-palace / empty-palace 行为
- schema/vector backend 的一致性

所以把 init 也做成清晰的 runtime，是很值得的一刀。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有改变 CLI / MCP `init` 表面
- 这次没有改 bootstrap 文件内容语义
- `service.rs` 后续仍然还能继续收口 `compress` 和 `doctor/prepare` 相邻入口层
