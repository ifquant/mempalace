# 背景

上一轮已经把 Rust MCP 的工具执行层从 `mcp.rs` 抽到了 `mcp_runtime.rs`，但新的问题也立刻出现了：`mcp_runtime.rs` 自己又变成了一个上千行的大文件，里面还是一整棵巨大的 `match name`。

这说明只做“一层抽取”还不够。要让 MCP 继续可维护，执行层本身也要按工具族分层。

# 主要目标

这次的目标是把 `mcp_runtime` 再按工具族切开：

- 读面工具单独放一层
- 写面和 maintenance 单独放一层
- project/bootstrap/helper 工具单独放一层
- registry 工具单独放一层

同时保持所有 MCP 外部行为不变。

# 改动概览

- 新增 `rust/src/mcp_runtime_read.rs`
- 新增 `rust/src/mcp_runtime_write.rs`
- 新增 `rust/src/mcp_runtime_project.rs`
- 新增 `rust/src/mcp_runtime_registry.rs`
- `rust/src/mcp_runtime.rs` 现在只保留：
  - `call_tool()` 顶层分发
  - `tool_error()`
  - `palace_exists()`
  - `best_effort_wal_log()`
- `rust/src/lib.rs` 导出新的 MCP runtime family 模块
- `rust/README.md` 同步说明新的 MCP family 分层

# 关键知识

这里的关键不是“拆更多文件”，而是把分发逻辑和工具族语义对齐。

比如：

- palace read / graph / KG read 这些工具，本质上都是“读面”
- add drawer / KG invalidate / diary write / repair / dedup 这些，本质上都是“写面或维护面”
- onboarding / normalize / split / instructions / hook 则更像“project helper”
- registry 是单独一条稳定子域

按这个边界切，后续加新 MCP 工具时，落点会更自然，不需要回头重新判断“这个工具该塞进那棵超级 match 的哪一段”。

# 补充知识

1. 做 dispatcher 重构时，保留一个非常薄的顶层总入口很有价值。这样调用者永远只认 `call_tool()`，内部怎么分模块以后都能继续调整。

2. 当一个模块已经完成“第一层抽取”后，下一步通常不是继续抽 helper，而是先按业务子域切开。这样比先做更多零散 util 更能降低认知负担。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有改动 MCP schema、tool catalog 或协议层
- 这次没有继续把 read/write family 内部再拆更细
- 这次没有改动 `python/`、`hooks/`、`docs/`、`assets/`、`.github/` 或其他子树
