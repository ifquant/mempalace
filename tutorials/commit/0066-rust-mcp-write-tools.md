# 背景

前面 Rust 版已经把 MCP 的只读面补得比较完整了：状态、搜索、图工具、KG 读面、diary 读写都已经有了。

但和 Python 版相比，还缺一大块真正会改 palace 数据的 MCP 工具：

- `mempalace_add_drawer`
- `mempalace_delete_drawer`
- `mempalace_kg_add`
- `mempalace_kg_invalidate`

如果这块不补，Rust 版就还是“能看不能写”，离 Python 的 MCP 使用方式还差一截。

# 主要目标

这次提交的目标是把上面四个 Python MCP 写工具整块迁到 Rust，并保证：

- 不是只在 `mcp.rs` 里临时拼 JSON
- service / storage / vector 三层都真正具备对应能力
- 成功路径和缺参错误路径都有集成测试
- `add_drawer` / `kg_add` 可以像 Python 一样在没有现成 palace 时自动初始化

# 改动概览

这次改动主要分四层。

第一层是 `model.rs`。

新增了这些结构化结果类型：

- `DrawerWriteResult`
- `DrawerDeleteResult`
- `KgWriteResult`
- `KgInvalidateResult`

这样 MCP、CLI、service 以后都能复用同一套返回结构，而不是每层各自造 JSON。

第二层是 `storage/sqlite.rs`。

新增了：

- `insert_drawer()`
- `delete_drawer()`
- `drawer_exists()`
- `invalidate_kg_triple()`

同时把 `add_kg_triple()` 改成返回结构化结果，而不是只返回 `()`

第三层是 `storage/vector.rs`。

新增了：

- `add_drawers()`
- `drawer_exists()`
- `delete_drawer()`

这样 Rust 的 drawer 写入/删除就不会只改 SQLite，而是会同步到 LanceDB。

第四层是 `service.rs` 和 `mcp.rs`。

service 新增了：

- `add_drawer()`
- `delete_drawer()`
- `kg_add()`
- `kg_invalidate()`

MCP 新增了工具注册、参数校验、错误提示和回包：

- `mempalace_add_drawer`
- `mempalace_delete_drawer`
- `mempalace_kg_add`
- `mempalace_kg_invalidate`

并且：

- `mempalace_add_drawer`
- `mempalace_kg_add`

现在会跳过“必须已有 palace”的前置拦截，允许首次写入时自动 bootstrap。

# 关键知识

## 1. Python 的 `sanitize_name()` 不是“强制归一化”，而是“保留原值做校验”

实现中一开始最容易犯的错，是把 Python 的 `sanitize_name()` 理解成“要把值变成 slug”。

实际上 Python 版做的是：

- 检查不能为空
- 检查不能有路径穿越
- 检查字符集安全
- 通过后保留原始文本

所以这次 Rust 也改成了同样思路：

- `wing`
- `room`
- `subject`
- `predicate`
- `object`

都会保留原始大小写和空格

真正需要 slug 的地方，只放在：

- `drawer_id`
- `mcp://...` 这类内部标识

这样对外兼容面才更像 Python。

## 2. 写向量库时，SQLite 和 LanceDB 不能只改一边

`add_drawer` / `delete_drawer` 这类操作如果只改 SQLite，会出现：

- `status` 看起来删掉了
- 但 `search` 还能搜到

或者反过来。

所以这次把 drawer 写面做成：

1. 先写/删 SQLite
2. 再同步写/删 LanceDB

这样 search 和状态统计不会分裂。

# 补充知识

## 1. 结果结构先固定，外层协议会轻松很多

如果一开始就让 storage/service 返回结构化类型，比如：

- `DrawerWriteResult`
- `KgInvalidateResult`

那 MCP、CLI、测试都会变简单。

反过来，如果底层只返回 `bool` 或 `()`，上层很快就会开始重复拼文案和字段。

## 2. “自动初始化”最好只放在明确安全的写工具上

这次没有把所有工具都改成自动建 palace。

只放开了：

- `mempalace_add_drawer`
- `mempalace_kg_add`
- 之前已有的 `mempalace_diary_write`

因为它们本身就是“往新 palace 写第一条数据”的典型入口。

而 `delete_drawer`、`kg_invalidate` 还是要求 palace 已存在，这样语义更稳。

# 验证

这次实际跑过：

```bash
cd rust
cargo fmt
cargo check
cargo test --test mcp_integration
```

重点覆盖了：

- `mcp_kg_write_tools_work`
- `mcp_kg_write_tools_return_tool_level_errors_for_missing_args`
- `mcp_add_and_delete_drawer_work`
- `mcp_drawer_write_tools_return_tool_level_errors_for_missing_args`

# 未覆盖项

这次还没有做这些：

- Python MCP 里的 write-ahead log / WAL 审计文件
- `delete_drawer` 后对 LanceDB 更深层的体积整理或 compact
- 更完整的 Python 写面，比如后续可能还有更细的 palace 管理工具
- 把这些新写工具接到 CLI 层；这次只收口了 MCP 面
