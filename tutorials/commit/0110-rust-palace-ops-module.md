# 背景

前几轮已经把 Rust 版的几条主线逐步从 `service.rs` 收出去：

- `miner`
- `palace_read`
- `registry_runtime`
- `embedding_runtime`

但 `service.rs` 里还保留着一整块“手工写操作 / project-local ops”：

- KG add/query/timeline/stats/invalidate
- manual drawer add/delete
- diary write/read

这些逻辑都带有明显的 palace runtime 特征：需要打开 SQLite、校验 embedding profile、在 drawer 写入时同时碰 SQLite + LanceDB。继续留在 `service.rs` 里，会让 `service` 重新积累具体存储编排细节。

# 主要目标

把这组手工 palace ops 抽成独立 `palace_ops` 模块，让 `service.rs` 继续退回到更薄的应用入口层。

# 改动概览

- 新增 [rust/src/palace_ops.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/palace_ops.rs)
- 提供 `PalaceOpsRuntime`
- 收进去的能力包括：
  - `add_kg_triple()`
  - `query_kg_raw()`
  - `kg_query()`
  - `kg_timeline()`
  - `kg_stats()`
  - `kg_add()`
  - `kg_invalidate()`
  - `add_drawer()`
  - `delete_drawer()`
  - `diary_write()`
  - `diary_read()`
- `service.rs` 对应方法现在只保留 thin wrapper，统一委托给 `palace_ops`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 为什么这里叫 `palace_ops`

这块不只是“写操作”，因为里面也有：

- `query_kg_raw`
- `kg_query`
- `kg_timeline`
- `kg_stats`
- `diary_read`

但它们和 `palace_read` 也不完全同类。`palace_read` 侧重：

- status
- search
- layers
- taxonomy / graph

而这块更像一组“手工操作 palace 的实用 runtime”，包括 diary、KG、manual drawer。  
所以这里用 `ops` 比 `write` 更准确。

## 为什么 `open_sqlite()` 也放在 `palace_ops`

KG / diary / drawer 这几条路径都需要：

- `ensure_dirs`
- open sqlite
- `init_schema`
- `ensure_embedding_profile`

这和 `palace_read` 一样，抽一个局部 runtime 入口比在每个方法里重复更稳。

## 为什么 drawer add 仍然在这里直接碰 LanceDB

manual drawer write 是少数明确的“双写”路径：

- SQLite 记录结构化 drawer
- LanceDB 保存 embedding 可检索副本

这层编排如果拆散到 `service`，模块化就失去意义了。所以这次刻意把这段双写逻辑保留在 `palace_ops`，而不是只抽一个半截 helper。

# 补充知识

当一个模块同时涉及：

- 输入校验
- 存储初始化
- 一次或两次后端写入
- 结果 shape 返回

它往往已经不只是“helper”，而是一个 runtime/facade。  
Rust 这里持续收口的重点，就是把这些 runtime 层一块块从 `service.rs` 剥离出来。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有改变 CLI / MCP 表面
- 这次没有改变 KG / diary / manual drawer 的行为语义
- 后续如果继续收口，`service.rs` 还可以再看 init/bootstrap/maintenance 那几段
