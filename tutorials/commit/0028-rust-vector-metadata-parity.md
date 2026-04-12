# 0028 Rust 向量层元数据对齐

本次提交把 Rust 版 MemPalace 的 **LanceDB 向量表** 继续往 Python 项目 miner 的元数据语义收紧。

## 这次做了什么

之前我们已经把这些 drawer 元数据写进了 SQLite：

- `source_file`
- `source_mtime`
- `added_by`
- `filed_at`

但 LanceDB 向量表里还只有：

- `id`
- `wing`
- `room`
- `source_path`
- `chunk_index`
- `text`
- `vector`

这样会有两个问题：

1. `search` 命中结果只能临时从 `source_path` 推导 `source_file`
2. 元数据分裂在 SQLite 和 LanceDB 两边，后面做过滤、调试、导出都不稳

所以这次把向量层也补齐了：

- 向量表 schema 新增 `source_file`
- 向量表 schema 新增 `source_mtime`
- 向量表 schema 新增 `added_by`
- 向量表 schema 新增 `filed_at`

同时 `search` 结果现在直接使用向量表中的持久化元数据，不再只靠运行时推导。

## 为什么不是直接重建 LanceDB 表

仓库里已经存在旧 palace。  
如果这次粗暴删表重建，会让已有向量数据直接丢失，这不符合当前 Rust 路线的“先可演进、再逐步收紧”原则。

所以这里用了 LanceDB 自带的 schema evolution：

- 打开旧表
- 检查缺哪些列
- 用 `add_columns` 原地补列

这样旧表就能平滑升级。

默认值策略是：

- `source_file = source_path`
- `source_mtime = NULL`
- `added_by = 'mempalace'`
- `filed_at = NULL`

这不完美，但比破坏性迁移稳得多。后续一旦重新 `mine`，这些字段就会被真实值覆盖。

## 这次补了哪些回归

- 新建 palace 后，`mine` 会把 Python 风格元数据同时写进 SQLite 和 LanceDB
- `search` 返回的 `source_file / source_mtime / added_by / filed_at` 来自向量层持久化字段
- 旧 LanceDB 表在 `init` 时会自动补齐缺失列

## 一个实现小知识点

LanceDB 的 schema evolution 不一定要整表重写。  
如果只是新增列，而且能接受用 SQL 表达式生成默认值，可以直接用：

- `table.add_columns(NewColumnTransform::SqlExpressions(...), None)`

这类“原地补列”特别适合做渐进式兼容。

## 另一个实现小知识点

即使已经把新列补进了表，读取时也最好保留 fallback。

原因是：

- 用户可能还没执行触发升级的命令
- 测试里可能会手工构造旧表
- 某些异常状态下列并不一定存在

所以 `search_hits_from_batch()` 里仍然保留了旧路径回退逻辑：

- 有 `source_file` 就直接读
- 没有时再从 `source_path` 推导

这样兼容性更稳。
