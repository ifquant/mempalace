# 背景

前面几轮 Rust 收口已经把很多顶层模块拆成了更清晰的能力边界，但 `rust/src/storage/sqlite.rs` 还是一个很典型的大文件。

它同时承担了这些职责：

- schema bootstrap 和 migration
- embedding profile 元数据检查
- drawer / compressed_drawer / ingested_files 读写
- taxonomy / room graph 读面
- knowledge graph 持久化
- diary 持久化

这些能力都基于同一个 `SqliteStore`，但变化原因其实不一样。如果继续把它们堆在一个文件里，后面再做本地存储层收口时，这里会重新变成一个新的阻塞点。

# 主要目标

这次提交的目标不是改 SQLite 行为，也不是改 schema，而是把 `sqlite.rs` 的内部职责按存储语义拆开，同时保持外部 `crate::storage::sqlite::*` 的使用方式不变。

也就是说：

- 调用方继续通过 `SqliteStore` 使用存储层
- `storage::sqlite` 仍然是统一入口
- 只是它内部不再把所有实现都塞在一个文件里

# 改动概览

这次主要新增了三个内部模块：

- `rust/src/storage/sqlite_schema.rs`
- `rust/src/storage/sqlite_drawers.rs`
- `rust/src/storage/sqlite_kg.rs`

拆分后的职责边界是：

## `sqlite_schema`

负责：

- `init_schema()`
- `schema_version()`
- `meta()` / `set_meta()`
- `ensure_embedding_profile()`
- 所有 migration helper
- schema bootstrap 和 migration record

也就是说，凡是“库长什么样”“版本怎么升级”“embedding profile 是否兼容”这类问题，都回到这里。

## `sqlite_drawers`

负责：

- `ingested_file_state()`
- `replace_source()`
- `insert_drawer()` / `delete_drawer()`
- `replace_compressed_drawers()`
- `list_drawers()` / `recent_drawers()`
- `list_wings()` / `list_rooms()` / `taxonomy()`
- `graph_room_rows()`

也就是 project mining、compression summary、taxonomy、room graph 这些和 drawer 数据直接相关的逻辑。

## `sqlite_kg`

负责：

- `add_kg_triple()`
- `invalidate_kg_triple()`
- `query_kg()` / `query_kg_entity()`
- `kg_timeline()` / `kg_stats()`
- `add_diary_entry()` / `read_diary_entries()`

也就是 knowledge graph 和 diary 这条“结构化事实 / agent note”支线。

## `sqlite`

现在只保留：

- `SqliteStore` 结构体
- 公共 record/type 定义
- `open()`
- `source_mtime()`
- 内部子模块声明

这样顶层入口依然很稳定，但内部职责已经不再混在一起。

# 关键知识

## 1. Rust 可以把同一个 `impl` 按文件拆开

这次拆分的关键点不是新建三个无关 struct，而是保留同一个 `SqliteStore`，然后在不同文件里继续写它的 `impl`。

这很适合“一个 runtime / store，对外是统一对象，但内部方法已经明显分家”的场景。

这样做的好处是：

- API 不需要重命名
- 调用方完全不用迁移
- 内部可以按能力拆开维护

## 2. 按“数据语义”拆，通常比按 CRUD 动作拆更稳

如果把方法机械地拆成：

- read
- write
- misc

短期可能看起来整齐，但后面很容易再次混乱。

这次更稳的边界是按数据语义：

- schema / migration
- drawers / compression / taxonomy
- KG / diary

因为这些分组更接近系统里的真实子领域。

# 补充知识

## 1. 内部重构优先保留稳定 facade，验证压力会小很多

这次外部 `SqliteStore` surface 没变，所以大部分测试天然就在帮我们验证：

- 旧调用点是否还通
- trait / runtime 之间的编排是否没断
- CLI / MCP 是否还走到同一套行为

这比一边拆文件、一边重塑 API 更容易控风险。

## 2. 大文件不一定要一次拆到最细

虽然 `sqlite_drawers` 里现在仍然有不少内容，但这次先做到“按主语义切开”就已经明显降复杂度了。

后面如果继续收口，还可以再细分，比如：

- drawer CRUD
- compressed_drawers
- taxonomy / graph read helpers

先做稳定的一刀，比一开始就追求最细颗粒度更靠谱。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

这次没有改这些内容：

- 没有改 SQLite schema 版本号
- 没有改变 migration 行为
- 没有改变 drawer / KG / diary 的外部 API
- 没有继续拆 `vector.rs`
- 没有改 Python 存储层实现
