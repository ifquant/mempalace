# 0065 Rust MCP 补齐 agent diary 读写面

这次继续按“大块收口”的方式，把 Python MCP 里的 agent diary 工具补到了 Rust：

- `mempalace_diary_write`
- `mempalace_diary_read`

## 这次为什么要单独做 diary

因为这不是普通的搜索工具。

它的职责是让 agent 有一条明确的“会后落账”通道：

- 写下这次做了什么
- 写下观察到的事情
- 写下后续可能重要的点

从协议角度看，这和 `status/search/kg_query` 一样重要，因为 `PALACE_PROTOCOL` 本身就在要求：

- after each session: `mempalace_diary_write`

## Rust 这次怎么实现

这轮没有先把 diary 塞回向量库，而是先做了一个稳定的 SQLite diary 表：

- `diary_entries`

字段包括：

- `agent_name`
- `wing`
- `room`
- `topic`
- `entry`
- `timestamp`
- `date`

对应 schema 版本从 `4` 升到了 `5`。

## 和 Python 的当前差异

Python 版的 diary 最终也会进 Chroma collection。

Rust 这轮先优先保证：

1. diary MCP 接口面可用
2. 数据能本地持久化
3. 读取语义和返回 shape 先对齐

也就是说，这轮先把“协议正确性”和“本地落盘”做稳，再决定是否让 diary 同时进入向量检索面。

## 这次补了哪些行为

### `mempalace_diary_write`

- 支持：
  - `agent_name`
  - `entry`
  - `topic`，默认 `general`
- 返回：
  - `success`
  - `entry_id`
  - `agent`
  - `topic`
  - `timestamp`

并且它现在可以在“palace 还没初始化”时自动初始化，不再被 MCP 的 no-palace 入口直接挡住。

### `mempalace_diary_read`

- 支持：
  - `agent_name`
  - `last_n`
- 返回：
  - `agent`
  - `entries`
  - `total`
  - `showing`
  - `message`

如果这个 agent 还没有 diary，会返回 Python 风格：

- `entries: []`
- `message: "No diary entries yet."`

## 这次补了哪些回归

- `tools/list` 暴露了 diary 两个工具
- `diary_write` 后能立刻 `diary_read`
- `diary_write` 会自动初始化 palace
- 新 agent 的 `diary_read` 会返回空列表和 message
- 缺参时，两个工具都会返回工具级 `error + hint`

## 顺手记一个知识点

当一个“写工具”承担的是操作协议角色，而不是搜索角色时，可以先选：

- 本地事务性落盘
- 稳定返回 shape
- 好迁移的 schema

不一定一开始就把它硬塞进向量检索链路。

先把“能安全写、能稳定读、不会丢”做对，通常比“先能搜”更重要。
