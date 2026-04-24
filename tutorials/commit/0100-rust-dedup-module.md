## 背景

Rust 版前面已经把不少 Python 对应能力抽成了独立库层：

- `entity_detector`
- `room_detector`
- `normalize`
- `palace_graph`
- `knowledge_graph`

但 dedup 这条线虽然已经能从 CLI/MCP 跑通，算法本身还埋在
[rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs) 里面，
包括：

- source 分组
- 向量相似度比较
- keep/delete 决策
- summary 组织

Python 侧有独立的 [python/mempalace/dedup.py](/Users/dev/workspace2/agents_research/mempalace/python/mempalace/dedup.py)，
所以这一轮继续按同样策略，把 dedup 也收成单独模块。

## 主要目标

- 给 Rust 新增独立 `dedup` 模块
- 把 dedup 规划逻辑从 `service.rs` 搬出去
- 提供显式 `Deduplicator` / `DedupPlan`
- 保持 CLI / MCP / service 外部行为不变
- 把这层库 API 写进 README

## 改动概览

- 新增 [rust/src/dedup.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/dedup.rs)
  - `Deduplicator::new()`
  - `Deduplicator::plan()`
  - `DedupPlan::into_summary()`
  - `cosine_distance()`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `dedup`
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `App::dedup()` 现在只负责：
    - 读 SQLite drawers
    - 读 LanceDB vectors
    - 调 `Deduplicator`
    - 必要时执行删除
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. dedup 有两层：规划和执行

真正需要抽出来的不是“删除数据库里哪些行”，而是 dedup 的核心规划：

- 同一个 `source_file` 里哪些 drawer 应该比较
- 哪些 vector 被视为 near-duplicate
- 哪些保留，哪些删除
- 最后怎么形成 summary

删除 SQLite / LanceDB 仍然是 service 层更合适，因为它掌握：

- palace path
- store 初始化
- 写操作时机

所以这轮的分层是：

- `dedup` 模块：计划
- `service`：执行

### 2. facade 抽取的重点是把“算法”从应用层剥开

service 适合做 orchestration，不适合长期承载越来越多领域算法。
如果 dedup 逻辑继续留在 `service.rs`，后面再加：

- 更复杂的 source 过滤
- richer keep rules
- stats-only 变化

service 只会更难维护。

这轮抽出 `Deduplicator` 之后，后续如果还要增强 dedup，入口就已经清楚了。

## 补充知识

### 1. `DedupPlan::into_summary()` 是一种很实用的边界设计

很多时候“算法结果”和“CLI/MCP 要回给用户的 JSON”不是一回事。

如果直接让算法模块产出最终 summary，算法就会反过来依赖：

- palace path
- version
- CLI 语义字段

这会把模块重新绑回应用层。

所以这里先做两步：

1. `plan()` 返回纯 dedup 结果
2. `into_summary()` 由外层补上 app 语义字段

这样边界更稳。

### 2. 抽算法模块时，把私有 helper 一起带走更干净

这轮把 `cosine_distance()` 一起搬进了 `dedup` 模块，而不是留在 service。
原因很简单：

- 这个函数只服务 dedup
- 它跟 service 的其它逻辑没有共享价值

如果 helper 留在旧文件里，模块边界表面上抽出来了，实质上还是互相缠着。

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

- `cosine_distance()` 对 identical vectors 返回 0
- `Deduplicator` 会保留更长 drawer，并把近重复项标记为删除
- 现有 service / CLI dedup 路径继续通过

## 未覆盖项

- 这次没有把 LanceDB / SQLite 的实际删除动作搬出 service
- 这次没有扩新的 dedup 策略，只先把现有算法模块化
- 这次没有改 Python `dedup.py`，只是把 Rust 的库层形状继续往它对齐
