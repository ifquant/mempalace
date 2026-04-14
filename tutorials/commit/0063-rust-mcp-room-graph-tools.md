# 0063 Rust MCP 补齐 room graph 三件套

这次是一整块兼容收口：把 Python MCP 里的 room graph 只读工具一起搬到了 Rust。

新增工具：

- `mempalace_traverse`
- `mempalace_find_tunnels`
- `mempalace_graph_stats`

## 这些工具本质上是什么

它们不是知识图谱。

它们是从 palace 里的 drawer 元数据临时建出来的一个“房间图”：

- 节点：`room`
- 连接条件：同一个 `room` 出现在多个 `wing`
- 遍历方式：按共享 wing 做 BFS

所以这层图更像“主题桥接图”，不是实体关系图。

## Rust 这次怎么实现

Rust 没有直接照搬 Python 的 Chroma metadata graph，而是：

1. 从 SQLite `drawers` 表读取 `room / wing / filed_at`
2. 在 service 层临时构建 room graph
3. MCP 工具直接复用这层 service

这样有两个好处：

- 不依赖向量表也能工作
- 图工具和 CLI/MCP 的其它读路径一样，统一走 SQLite 元数据

## 和 Python 的一个差异

Python 图层里有 `hall` 和 `date`。
Rust 当前 drawer 元数据里没有 `hall`，所以这轮选择的是：

- shape 尽量兼容
- `halls` 先稳定返回空数组
- `recent` 用 `filed_at` 近似

也就是说，这轮优先保证 agent 接口面和主要语义对齐，不伪造不存在的数据。

## 这次补了哪些回归

- `tools/list` 暴露了新图工具
- `mempalace_traverse` 可以真的遍历到跨 wing 的 shared room
- `mempalace_find_tunnels` 能找到 tunnel room
- `mempalace_graph_stats` 能返回总 room 数、tunnel 数、top tunnels
- `start_room` 缺失时，`mempalace_traverse` 返回工具级 `error + hint`
- room 不存在时，`mempalace_traverse` 返回 Python 风格的 `error + suggestions`

## 顺手记一个知识点

这类“图工具”很多时候没必要一开始就引入图数据库。

如果你的关系其实是：

- 现有结构化元数据
- 少量派生边
- 只读分析查询

那么在 service 层临时建图，通常更简单、也更容易保持和主存储一致。
