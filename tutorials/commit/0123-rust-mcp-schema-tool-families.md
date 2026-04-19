# 背景

前一轮已经把 Rust MCP 的 runtime 按工具族切成了 `read / write / project / registry` 四块，但 schema 这一侧还停留在老结构：`mcp_schema.rs` 里放着一整份巨大的 tool catalog，同时还混着 protocol negotiation、no-palace policy、参数 coercion 和字符串 helper。

这会带来一个明显问题：runtime 和 schema 的边界不对称。读执行层时已经能按工具族定位，但读 schema 时还得回到一个大文件里翻。

# 主要目标

这次的目标是把 Rust MCP 的 schema/catalog 也做成和 runtime 一致的 family 分层：

- tool catalog 按 read / write / project / registry 切开
- protocol / coercion / helper 收到单独 support 模块
- `mcp_schema.rs` 只保留薄入口和聚合导出

# 改动概览

- 新增 `rust/src/mcp_schema_catalog_read.rs`
- 新增 `rust/src/mcp_schema_catalog_write.rs`
- 新增 `rust/src/mcp_schema_catalog_project.rs`
- 新增 `rust/src/mcp_schema_catalog_registry.rs`
- 新增 `rust/src/mcp_schema_support.rs`
- `rust/src/mcp_schema.rs` 现在只负责：
  - 聚合四组 tool catalog
  - re-export protocol / helper API
  - 保留通用 `tool()` builder
- `rust/src/lib.rs` 导出新增 schema family 模块
- `rust/README.md` 同步说明新的 MCP schema 分层

# 关键知识

这里最重要的是“让结构两边对称”：

- `mcp_runtime_read` 对应 `mcp_schema_catalog_read`
- `mcp_runtime_write` 对应 `mcp_schema_catalog_write`
- `mcp_runtime_project` 对应 `mcp_schema_catalog_project`
- `mcp_runtime_registry` 对应 `mcp_schema_catalog_registry`

这样以后新增工具时，调用链非常直接：

1. 在对应 family 的 schema 文件里加 tool schema
2. 在对应 family 的 runtime 文件里加执行逻辑

不再需要在一个大 schema 文件里滚动查找，也不容易把 schema 和 runtime 落到不同语义分组里。

# 补充知识

1. 当一个系统既有“声明面”又有“执行面”时，让两边的模块边界保持一致，通常比单独优化某一边更重要。因为维护者真正需要的是“能顺着同一语义边界走完整条链路”。

2. 把 `support` 类逻辑单独收出来的好处，不只是减小文件长度，还能防止 catalog 模块被一堆 coercion / helper 噪声淹没。这样看 schema 时更接近“声明清单”，看 support 时更接近“策略和规则”。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有改动任何 MCP 外部 tool 名、参数 schema 语义或协议行为
- 这次没有继续把 `mcp.rs` transport 层再拆更细
- 这次没有改动 `python/`、`hooks/`、`docs/`、`assets/`、`.github/` 或其他子树
