# 背景

前一轮已经把 `rust/src/storage/sqlite.rs` 按 schema / drawers / KG 语义拆开了，但 `rust/src/storage/vector.rs` 还是把 LanceDB 这条线的所有事情都放在一个文件里：

- table bootstrap
- legacy metadata 列补齐
- Arrow `RecordBatch` 编码
- search hit 解码
- CRUD / search 查询路径

它虽然比 `sqlite.rs` 小一些，但职责已经明显混在一起了。如果继续放着不拆，后面存储层的局部演进又会重新集中回一个文件里。

# 主要目标

这次提交的目标是把 `vector.rs` 的内部职责继续按“schema / batch / query”三层拆开，同时保持外部 `crate::storage::vector::*` surface 不变。

也就是说：

- 调用方继续通过 `VectorStore` 使用 LanceDB
- 顶层模块名仍然叫 `storage::vector`
- 只是内部实现不再全部塞在 `vector.rs`

# 改动概览

这次新增了三个内部模块：

- `rust/src/storage/vector_schema.rs`
- `rust/src/storage/vector_batch.rs`
- `rust/src/storage/vector_query.rs`

拆分后的职责边界是：

## `vector_schema`

负责：

- `TABLE_NAME`
- `schema(dimension)`
- `ensure_table()`
- `ensure_metadata_columns()`

也就是 LanceDB 表应该长什么样、已有表缺哪些 metadata 列、需要怎样做 legacy 列升级，这些都回到这一层。

## `vector_batch`

负责：

- `record_batch()`
- `vector_drawers_from_batch()`
- `vector_from_row()`

也就是 Arrow 编码/解码本身。

这层只关心“怎么把 drawer 和 embedding 变成 batch”，以及“怎么把 batch 还原回向量 drawer”，不关心查询语义。

## `vector_query`

负责：

- `replace_source()`
- `search()`
- `add_drawers()`
- `drawer_exists()`
- `delete_drawer()` / `delete_drawers()`
- `clear_table()`
- `list_drawers()`
- `search_hits_from_batch()`
- `filter_sql()` / `filter_source_sql()`

也就是所有真正面向 LanceDB 查询/写入的路径，以及 search hit 的解码逻辑。

## `vector`

现在只保留：

- `VectorDrawer`
- `VectorStore`
- `connect()`
- 内部模块声明

顶层 facade 继续很薄，外部 surface 保持稳定。

# 关键知识

## 1. 向量存储层通常天然分成 schema / batch / query 三块

这类模块很容易一开始就长成大文件，因为它们看上去都“跟向量库有关”。

但从维护角度看，最自然的切法通常是：

- schema：表结构和升级
- batch：编码和解码
- query：读写和检索路径

这三类代码的变化原因不同，拆开之后更容易做局部修改。

## 2. Arrow 编码逻辑单独放出来，后面更容易定位数据问题

`record_batch()` 和 `vector_drawers_from_batch()` 这种函数，本质上是存储层的数据边界。

把它们独立出来的好处是：

- 以后如果列顺序、类型、nullable 规则有问题，更容易集中定位
- schema 变化时，可以更清楚地对照 batch 编码是否同步
- query 层就不用再夹杂太多“列怎么转 Arrow”这种噪声

# 补充知识

## 1. facade + 内部 helper 模块，是控制重构风险的好手法

这次没有改 `VectorStore` 的公开方法名，也没有让上层代码改 import。

所以验证时，大多数现有测试天然就在帮我们确认：

- facade 还在
- query 路径没断
- batch 编码没错位
- LanceDB 的 legacy metadata 补列逻辑没丢

这种做法特别适合“只想继续收口内部结构，不想同时制造外部迁移成本”的阶段。

## 2. 同一轮里连续收 SQLite 和 LanceDB，能让存储层边界更对称

前一轮把 SQLite 按语义切开，这一轮把 LanceDB 也按 schema / batch / query 切开，结果是整个 `storage/` 目录的阅读体验会更一致：

- SQLite 有自己的 family split
- LanceDB 也有自己的 family split

这种对称性本身就是一种工程质量收益，尤其对后来的维护者很重要。

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

- 没有改变 LanceDB table schema 的外部语义
- 没有改变 search / add / delete / replace 的行为
- 没有改变相似度分数的计算方式
- 没有继续拆 `vector_query` 内部的 search hit decode helper
- 没有改 Python 向量存储实现
