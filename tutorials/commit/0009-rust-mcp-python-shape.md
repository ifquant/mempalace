# 背景

Rust 版之前已经有只读 MCP 工具，但它和 Python 版还有几个明显差距：

- `tools/list` 里的 `inputSchema` 基本是空壳
- `mempalace_status` 没带 Python 版最关键的 `protocol` 和 `aaak_dialect`
- `mempalace_search` 返回 shape 偏 Rust 内部结构，不像 Python 的 MCP 输出
- 没 palace 时更像内部错误，而不是 Python 那种稳定的 `error + hint`

这些差距不会让功能“不能用”，但会让依赖 Python 行为的 agent 提示和上层调用更难平滑迁移。

# 主要目标

这次提交的目标是把 Rust 只读 MCP 的外部 shape 再往 Python 版收紧一层：

1. 对齐 read tools 的 `inputSchema`
2. 对齐 `status` 的 wake-up 信息
3. 对齐 `search` 的返回结构
4. 对齐“无 palace”时的错误返回

# 改动概览

主要改动如下：

- `rust/src/mcp.rs`
  - `SUPPORTED_PROTOCOL_VERSIONS` 扩展到和 Python 版同一组：
    - `2025-11-25`
    - `2025-06-18`
    - `2025-03-26`
    - `2024-11-05`
  - `tools/list` 不再只返回空 schema
  - 为这些只读工具补了 Python 风格 `inputSchema`：
    - `mempalace_status`
    - `mempalace_list_wings`
    - `mempalace_list_rooms`
    - `mempalace_get_taxonomy`
    - `mempalace_search`
  - `mempalace_status` 现在返回：
    - `total_drawers`
    - `wings`
    - `rooms`
    - `palace_path`
    - `protocol`
    - `aaak_dialect`
  - `mempalace_search` 现在返回 Python 风格：
    - `query`
    - `filters`
    - `results`
  - 每条 search result 现在包含：
    - `text`
    - `wing`
    - `room`
    - `source_file`
    - `similarity`
  - `tools/call` 现在会把结果输出成 pretty JSON，更接近 Python
  - `limit` 参数增加类型收敛，允许字符串/数字输入
  - 没 palace 时返回：
    - `{"error":"No palace found","hint":"Run: mempalace init <dir> && mempalace mine <dir>"}`
- `rust/tests/mcp_integration.rs`
  - 增加 schema 断言
  - 增加 `status` 字段断言
  - 增加 Python 风格 no-palace 返回断言
- `rust/README.md`
  - 补充当前 MCP 兼容说明

# 关键知识

## 1. MCP 兼容不只是“工具名一样”

一个常见误区是觉得：

- 工具名一样
- 参数大概差不多

就算兼容了。

但对 agent 来说，真正关键的是：

- `tools/list` 里暴露了什么 schema
- 首次 wake-up 能拿到什么上下文
- 错误返回是不是稳定 shape

这也是为什么这次重点不是多加工具，而是把现有 read tools 的外部契约收紧。

## 2. `status` 里的协议文本其实是运行时提示面的一部分

Python 版把 `protocol` 和 `aaak_dialect` 放进 `mempalace_status`，不是随手塞文案。  
它的作用是让上层 agent 在第一次调用状态工具时就学到：

- 如何先查再答
- 什么时候写 diary
- AAAK 是什么格式

如果 Rust 不提供这些字段，功能上能跑，但 agent 行为会比 Python 路线弱一截。

# 补充知识

## 为什么这里把 `search` 映射成 MCP 专用 shape，而不是改 service 层模型

因为 service 层现在承担的是 Rust 内部 API：

- CLI
- 测试
- MCP

而 Python 风格的 `search_memories()` shape 更像一个外部接口适配层。  
把这种兼容逻辑放在 `mcp.rs`，可以避免为了协议兼容去污染内部模型。

## 为什么“无 palace”要返回稳定 JSON，而不是直接报异常

对人类 CLI 来说，异常提示也许够用。  
但对 MCP 客户端和 agent 来说，更重要的是：

- 可以程序化判断
- 可以看到明确 hint
- 不会因为内部错误类型变化而破坏调用逻辑

所以这类返回 shape 的稳定性，本身就是接口设计的一部分。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增验证点：

- `mcp_read_tools_work`
  - 验证 `tools/list` 的 schema
  - 验证 `status` 含 `protocol/aaak_dialect`
  - 验证 `search` 含 `query/filters/source_file/similarity`
- `mcp_read_tools_return_python_style_no_palace_response`
  - 验证无 palace 时返回 Python 风格 `error + hint`

# 未覆盖项

这次没有继续做：

- Python MCP 的写工具
- `mempalace_get_aaak_spec`
- KG / traverse / tunnel / diary 工具
- 更严格的逐字节返回兼容

所以这次提交的定位是：  
先把 Rust 只读 MCP 的“工具契约”和“结果 shape”继续往 Python 版贴近，而不是扩展到全部 MCP 能力。
