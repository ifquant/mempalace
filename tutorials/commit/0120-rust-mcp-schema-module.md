# 背景

前面已经把 Rust CLI、runtime、library 侧很多边界收开了，但 `mcp.rs` 仍然还是一个很重的单文件：

- 协议版本协商
- MCP 工具清单
- 每个工具的 input schema
- 参数 coercion
- required-arg helper
- tool 执行逻辑

这些内容里，至少有一半不是“执行工具”，而是“定义工具长什么样”。继续把它们堆在一个文件里，会让 `mcp.rs` 同时承担 schema 和 runtime 两种责任。

## 主要目标

- 先把 `mcp` 的 schema / catalog / argument helper 从执行逻辑里拆出去
- 让 `mcp.rs` 更集中在：
  - request handling
  - stdio loop
  - tool execution
- 不改变任何 MCP tool name、参数面或返回语义

## 改动概览

- 新增 `rust/src/mcp_schema.rs`
  - 提供 `SUPPORTED_PROTOCOL_VERSIONS`
  - 提供 `PALACE_PROTOCOL`
  - 提供 `tools()`
  - 提供 `negotiate_protocol()`
  - 提供 `coerce_argument_types()`
  - 提供 `required_str()` / `string_list_arg()`
  - 提供 `requires_existing_palace()` / `no_palace()`
  - 提供 `truncate_duplicate_content()`
- 更新 `rust/src/mcp.rs`
  - 从 `mcp_schema` 引入协议/schema/helper
  - 删除原来内联在 `mcp.rs` 里的 catalog 和参数 coercion/helper 实现
  - 保留 `call_tool()` 执行面在原文件
- 更新 `rust/src/lib.rs`
  - 导出 `mcp_schema`
- 更新 `rust/README.md`
  - 把 `mcp_schema` 记成 Rust library structure 的一部分

## 关键知识

### MCP 也有“schema 层”和“执行层”

这轮的核心不是简单地“把代码挪文件”，而是明确 `mcp` 侧有两类完全不同的东西：

- schema/catalog
  - 工具名
  - input schema
  - 协议版本
  - 参数 coercion 规则
- execution/runtime
  - `tools/call`
  - `call_tool()`
  - stdio server loop

一旦把这两类东西拆开，后面如果还要继续收口 `mcp.rs`，就更容易沿着“执行层继续拆”往下走。

### 参数 coercion 适合做成共享 helper，而不是散在各工具分支里

`coerce_argument_types()` 里其实是一套“把字符串参数纠成 bool/u64/f64”的通用规则。  
它本质上更接近 schema 约束，而不是业务执行。

把它收进 `mcp_schema` 的好处是：

- 工具参数面更集中
- 后续新增 MCP tool 时，更容易在一个地方补 schema 和 coercion
- `mcp.rs` 里的执行逻辑更少被输入清洗细节打断

## 补充知识

### 大文件收口时，先拆“稳定定义”，再拆“易变执行”

这轮没有直接把 `call_tool()` 巨大的 match 一次拆碎，而是先抽：

- tool catalog
- protocol constants
- coercion helpers

因为这些部分通常更稳定，也更容易独立验证。  
真正的执行逻辑变化面更大，适合放到下一轮再继续切。

### `README` 同步不仅是文档工作，也是架构事实登记

这个仓库一直在把“模块边界”同步写进 `rust/README.md`。  
这不是装饰，而是在持续把 repo 事实从“聊天里知道”变成“代码库里看得到”。

对于后续 agent 或新人来说，看到 `mcp_schema` 出现在 README 里，就能更快理解：

- schema 已经独立
- `mcp.rs` 主要保留执行面

## 验证

在 `rust/` 下执行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这轮没有继续拆 `call_tool()` 的大 match
- 这轮没有改变任何 MCP tool name、input schema 或返回 payload
- 这轮只是把 schema/helper 从 `mcp.rs` 收口成独立模块
