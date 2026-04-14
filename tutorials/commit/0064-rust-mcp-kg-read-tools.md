# 0064 Rust MCP 补齐 KG 只读三件套

这次继续按“大块收口”的方式，把 Python MCP 里的知识图谱只读工具一起补到了 Rust：

- `mempalace_kg_query`
- `mempalace_kg_timeline`
- `mempalace_kg_stats`

## 这次补的是哪一层

不是新增写能力。

这次只补“读面”：

- 按实体查关系
- 按时间看事实时间线
- 看整个 KG 的统计摘要

这样 Rust MCP 现在已经不只是：

- palace drawer 检索
- taxonomy / graph 只读

还开始覆盖 Python 的 KG 查询面。

## Rust 这次怎么做

Rust 现有 `kg_triples` 表已经有：

- `subject`
- `predicate`
- `object`
- `valid_from`
- `valid_to`

所以这轮没有先改 schema，而是直接在 SQLite 查询层补：

1. `query_kg_entity(entity, as_of, direction)`
2. `kg_timeline(entity)`
3. `kg_stats()`

然后 service 和 MCP 只是复用这层结果。

## 和 Python 的一个实现差异

Python 的 `knowledge_graph.py` 有 `entities` 表。
Rust 当前没有单独的实体表，所以 `kg_stats.entities` 这轮是通过：

- `subject`
- `object`

做 `UNION` 去重得到的。

这在当前 Rust 结构里已经足够表达“图里一共有多少实体名字”，而且不需要为了只读统计先引入新 schema。

## 这次补了哪些语义

### `mempalace_kg_query`

- 支持 `entity`
- 支持 `as_of`
- 支持 `direction = outgoing | incoming | both`
- 返回：
  - `entity`
  - `as_of`
  - `facts`
  - `count`

### `mempalace_kg_timeline`

- 支持按实体过滤
- 也支持全量时间线
- 返回：
  - `entity`
  - `timeline`
  - `count`

### `mempalace_kg_stats`

- 返回：
  - `entities`
  - `triples`
  - `current_facts`
  - `expired_facts`
  - `relationship_types`

## 这次补了哪些回归

- `tools/list` 现在暴露 KG 三个只读工具
- `kg_query` 的 `direction/as_of` 主链路可用
- `kg_timeline` 能返回实体时间线
- `kg_stats` 能返回统计摘要
- 非法 `direction` 会返回工具级 `error + hint`

## 顺手记一个知识点

如果你已经有了时间区间字段：

- `valid_from`
- `valid_to`

那么很多“时间知识图谱”的读操作，其实先不用图数据库也能做得很像：

- 当前事实：`valid_to IS NULL`
- 某日事实：`valid_from <= as_of <= valid_to`
- 时间线：按 `valid_from` 排序

也就是说，KG 的“时间语义”很多时候先是查询设计问题，不一定先是存储引擎问题。
