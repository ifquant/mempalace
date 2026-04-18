## 背景

Rust 版前面已经把几条 Python 模块线陆续收成了独立库层：

- `entity_detector`
- `room_detector`
- `normalize`
- `palace_graph`

但 KG 这条线虽然功能早就有了，实际读写逻辑还是挂在
[rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs) 和
[rust/src/storage/sqlite.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/storage/sqlite.rs)
之间。

Python 侧这块是独立的 [python/mempalace/knowledge_graph.py](/Users/dev/workspace2/agents_research/mempalace/python/mempalace/knowledge_graph.py)，
有清晰的 `KnowledgeGraph` 入口。如果 Rust 也要继续形成可编程库层，而不只是“service 上有几个 KG 方法”，
这条线也应该被抽出来。

## 主要目标

- 给 Rust 新增独立 `knowledge_graph` 模块
- 提供显式 `KnowledgeGraph` facade
- 让 `service` 的 KG 读写方法改成委托给这层
- 保持 CLI / MCP / SQLite 行为不变
- 把这层库 API 写进 README

## 改动概览

- 新增 [rust/src/knowledge_graph.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/knowledge_graph.rs)
  - `KnowledgeGraph::new()`
  - `add_triple()`
  - `invalidate()`
  - `query_raw()`
  - `query_entity()`
  - `timeline()`
  - `stats()`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `knowledge_graph`
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `add_kg_triple/query_kg/kg_query/kg_timeline/kg_stats/kg_add/kg_invalidate` 改为委托给新模块
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. storage 和 domain facade 不是一回事

`SqliteStore` 负责的是：

- 打开数据库
- 执行 SQL
- 返回低层结果

而 `KnowledgeGraph` facade 负责的是：

- 把 KG 读写接口组织成可复用的领域 API
- 为 service / 未来库调用方提供清晰入口
- 让 Rust 的模块 shape 往 Python `KnowledgeGraph` 靠

所以这轮不是“重复包装一层没用的壳”，而是在补领域边界。

### 2. service 继续保留对外入口，但不再持有 KG 细节

这轮没有删掉：

- `App::kg_query()`
- `App::kg_timeline()`
- `App::kg_stats()`
- `App::kg_add()`
- `App::kg_invalidate()`

只是把内部改成：

1. `open sqlite`
2. `KnowledgeGraph::new(&sqlite)`
3. 调 facade

这样现有 CLI / MCP / tests 都不用跟着大改，风险最低。

## 补充知识

### 1. facade 抽取适合从“最稳定的领域对象”开始

KG 这条线之所以适合现在抽，是因为它已经有很稳定的对象边界：

- triple
- fact
- timeline
- stats

像这种边界清楚的领域，比起直接去抽一个超大的 `service helper`，更适合先形成独立模块。

### 2. Rust 对齐 Python，不一定要照搬存储实现，但要照顾调用心智

Python `KnowledgeGraph` 自己管 SQLite 连接，Rust 这边为了复用现有 `SqliteStore`，
做成了 `KnowledgeGraph<'a> { store: &'a SqliteStore }`。

这不是一比一照抄实现，而是：

- 保留 Rust 现有存储分层
- 同时让调用方感受到“我在用一个 KG 模块”，而不是散落的 service/sqlite 函数

## 验证

实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

本次新增/保留覆盖了：

- `knowledge_graph` 的 triple round-trip
- invalidate 后 timeline / stats 行为
- 现有 service / MCP / CLI 的 KG 读写回归继续通过

## 未覆盖项

- 这次没有把 SQLite 里的 KG SQL 也搬出 `storage/sqlite.rs`
- 这次没有新增独立 KG CLI facade，只先把库层模块立住
- 这次没有改 Python `knowledge_graph.py`，只是把 Rust 的库层形状继续往它对齐
