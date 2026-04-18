## 背景

Rust 版前面已经把几条重要的 Python 模块线逐步抽成了独立库层：

- `entity_detector`
- `room_detector`
- `normalize`

但 room graph 这条线还停留在旧状态：虽然 CLI 和 MCP 都已经能用

- `traverse`
- `find_tunnels`
- `graph_stats`

实际算法却仍然塞在 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs) 里面。

Python 版这块有独立的 [python/mempalace/palace_graph.py](/Users/dev/workspace2/agents_research/mempalace/python/mempalace/palace_graph.py)，
所以如果 Rust 也想真正形成可编程库层，而不是只有“service 上能跑”，这条线也应该抽出来。

## 主要目标

- 给 Rust 新增独立 `palace_graph` 模块
- 把 room graph 构建、BFS 遍历、tunnel 检测、graph stats 从 `service.rs` 搬出去
- 保持 CLI / MCP / service 行为不变
- 把这层库 API 写进 README

## 改动概览

- 新增 [rust/src/palace_graph.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/palace_graph.rs)
  - `build_room_graph()`
  - `traverse_graph()`
  - `find_tunnels()`
  - `graph_stats()`
  - `fuzzy_match_room()`
  - `RoomGraph` / `GraphNodeData`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `palace_graph`
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `traverse_graph()` / `find_tunnels()` / `graph_stats()` 改委托给新模块
  - 删除内嵌 graph 结构和算法
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. service 不是算法仓库

`service.rs` 适合放“应用层入口”：

- 打开 SQLite
- 校验 embedding profile
- 组织 CLI / MCP 所需返回结构

但像 room graph 这种算法本身：

- 有自己的输入数据结构
- 有自己的遍历和排序规则
- 可以被多个调用方复用

继续塞在 `service.rs` 里，只会让 service 越来越像一个“大杂烩文件”。

### 2. graph 这条线天然适合独立模块

和普通 CRUD 不同，graph 逻辑本来就有清晰边界：

- graph build
- BFS traverse
- tunnel query
- stats summary

这类能力拆出来以后，不只是“文件更短”，更重要的是：

- 以后 CLI / MCP / layer API / 未来库调用方都可以共用
- 可以独立写模块测试
- service 不需要再挟带 graph 私有结构

## 补充知识

### 1. 抽模块时优先保留 service 外壳，风险最低

这次没有去改 service 的对外接口：

- `App::traverse_graph()`
- `App::find_tunnels()`
- `App::graph_stats()`

它们还是存在，只是内部改成：

1. 取 SQLite rows
2. 调 `palace_graph`
3. 回传结果

这样 CLI / MCP / 测试都不用大改，风险会比“一次性重排所有调用层”小很多。

### 2. 先做模块抽取，再做 library facade，更稳

如果后面真的要继续往 Python `palace_graph.py` 的“库级直调”方向推进，
最稳的顺序是：

1. 先把算法从 service 抽出来
2. 再决定要不要加更显式的 facade

否则很容易一上来就把“抽模块”和“重设 API”混到一起，回归面会变大。

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

- `palace_graph` 的 BFS 遍历结果
- `palace_graph` 的 tunnel 过滤
- 现有 CLI / MCP / service graph 路径继续通过

## 未覆盖项

- 这次没有新增独立 CLI `graph` facade，只先把库层模块立住
- 这次没有扩 graph 元数据，比如补 Python 版更完整的 `dates/halls` 输出语义
- 这次没有改 Python `palace_graph.py`，只是把 Rust 的库层形状继续往它对齐
