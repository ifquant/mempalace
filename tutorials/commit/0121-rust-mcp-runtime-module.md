# 背景

前一轮已经把 Rust MCP 的 schema、catalog 和参数 coercion 从 `mcp.rs` 里拆到了 `mcp_schema.rs`，但 `mcp.rs` 仍然塞着一整块 `call_tool()` 执行逻辑。这样的问题很直接：协议入口、stdio loop、工具执行、WAL 记录和 tool-level error 都混在一个文件里，继续加工具时会让边界重新变糊。

# 主要目标

这次的目标是继续把 Rust MCP 往更清晰的分层推进：

- `mcp.rs` 只保留协议入口和 stdio transport
- 把工具执行/runtime 整块移到独立模块
- 保持现有 MCP 行为完全不变

# 改动概览

- 新增 `rust/src/mcp_runtime.rs`
- 把 `call_tool()` 从 `rust/src/mcp.rs` 移到 `rust/src/mcp_runtime.rs`
- 把 MCP 共用的 `tool_error()`、`palace_exists()`、`best_effort_wal_log()` 一起移到 `mcp_runtime`
- `rust/src/mcp.rs` 现在只负责：
  - `handle_request()`
  - `run_stdio()`
  - 调用 `mcp_schema` 做 catalog/protocol 相关工作
  - 调用 `mcp_runtime::call_tool()` 执行具体工具
- 更新 `rust/src/lib.rs` 导出新模块
- 更新 `rust/README.md` 说明新的 MCP runtime 分层

# 关键知识

Rust 里这种拆法的关键不是“文件更短”本身，而是职责边界更稳定：

- `mcp_schema` 负责“工具长什么样”
- `mcp_runtime` 负责“工具怎么执行”
- `mcp.rs` 负责“请求怎么进来、响应怎么出去”

这样后面再继续扩 MCP 时，就不容易把协议层、执行层和工具 catalog 再次揉回一个文件。

另一个关键点是这次没有改动任何 MCP 外部行为。所有 tool 名、参数、错误 payload、WAL best-effort 写入逻辑都保持不变，所以这是一次结构收口，不是语义改写。

# 补充知识

1. 做大文件拆分时，先拆“边界最完整的一块”通常比按长度平均切更稳。这里 `call_tool()` 本身就是天然边界，所以适合整块搬走。

2. Rust 重构里如果目标是“纯结构调整”，最好让旧入口变成 thin wrapper，而不是一边拆一边改行为。这样验证成本低，回归也更容易判断。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有继续拆 `mcp_runtime.rs` 内部更细的按工具族分发
- 这次没有改 MCP tool catalog、参数 schema 或任何外部协议行为
- 这次没有动 `python/`、`hooks/`、`docs/`、`assets/`、`.github/` 或其他子树
