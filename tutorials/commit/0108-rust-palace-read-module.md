# 背景

在前几轮里，Rust 版已经把 `searcher`、`layers`、`palace_graph` 等读面 helper 抽成了独立模块，但 `service.rs` 里仍然保留了一大段 read-side palace surfacing 逻辑：

- `status`
- `list_wings`
- `list_rooms`
- `taxonomy`
- `traverse_graph`
- `find_tunnels`
- `graph_stats`
- `search`
- `wake_up`
- `recall`
- `layer_status`

这些方法本质上都属于“读取 palace、拼装摘要、返回 CLI/MCP 会消费的结果”，继续堆在 `service.rs` 里会让 `service` 重新变回大杂烩。

# 主要目标

把 Rust 的 read-side palace surface 抽成一个独立 `palace_read` 模块，同时保持现有 CLI / MCP / library 行为不变。

# 改动概览

- 新增 [rust/src/palace_read.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/palace_read.rs)
- 引入 `PalaceReadRuntime`
- 把下面这些能力迁入 `palace_read`：
  - `status()`
  - `list_wings()`
  - `list_rooms()`
  - `taxonomy()`
  - `traverse_graph()`
  - `find_tunnels()`
  - `graph_stats()`
  - `search()`
  - `wake_up()`
  - `recall()`
  - `layer_status()`
- `service.rs` 对应方法现在只保留 thin wrapper，统一委托给 `PalaceReadRuntime`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs) 导出新模块
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 为什么这里叫 `palace_read` 而不是 `query`

这块不只是数据库 query。它同时承担了：

- SQLite / LanceDB 的只读访问
- identity / layer 文本拼装
- graph summary 组装
- search result summary 组装

如果叫 `query`，会让模块名过窄；`palace_read` 更符合它的职责边界：围绕 palace 的读面 facade。

## 为什么 `open_sqlite()` 放在模块内部

这里反复出现的样板是：

- `ensure_dirs`
- `open sqlite`
- `init_schema`
- `ensure_embedding_profile`

把它收成 `PalaceReadRuntime::open_sqlite()` 后：

- 读面入口不再每个方法手写一遍
- embedding profile guard 不会因为复制粘贴而漏掉
- 后续如果 read-side 初始化策略变化，只需要改一处

## 为什么 `search()` 仍然先打开 SQLite

虽然真正检索结果来自 LanceDB，但 Rust 这里仍然先走 `open_sqlite()`：

- 维持和其他读命令一致的 palace/runtime 校验
- 保持 broken-sqlite 场景下的错误 surface 不漂移
- 不让 `search` 变成一个绕过 palace 基本健康检查的特殊入口

# 补充知识

一个常见的 Rust 模块化误区是：只抽底层 helper，不抽“中层 orchestration facade”。这样最后会得到很多小模块，但 `service.rs` 仍然是最重的地方。  
这轮的重点不是再发明新 helper，而是把一整段“读面 orchestrator”整体搬出去，让 `service` 真正退回到应用入口层。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有改 CLI / MCP 行为
- 这次没有改 search / wake-up / recall 的结果 shape
- `service.rs` 仍然还有 registry、maintenance、write-side 等块，后续还可以继续收口
