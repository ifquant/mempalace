## 背景

前一轮已经把 Rust 的 maintenance / AAAK 工具接进了 MCP，但还有一块明显断层：

- `onboarding`
- `normalize`
- `split`

这三个工具都已经有稳定的 Rust 库入口和 CLI，但 MCP host 还不能直接调用。这样会让“项目本地 world bootstrap”和“conversation 预处理”只能在终端里做，代理自动化无法接上。

## 主要目标

- 给 Rust MCP 新增 `mempalace_onboarding`
- 给 Rust MCP 新增 `mempalace_normalize`
- 给 Rust MCP 新增 `mempalace_split`
- 让这三条工具直接复用已有库入口，而不是重写 CLI 逻辑
- 补齐 MCP integration test、README 和教程

## 改动概览

- 在 `rust/src/mcp.rs` 的 `tools()` 里新增 3 条 schema：
  - `mempalace_onboarding`
  - `mempalace_normalize`
  - `mempalace_split`
- 在 `call_tool()` 里直接接到已有库函数：
  - `run_onboarding()`
  - `normalize_conversation_file()`
  - `split::split_directory()`
- 新增 `string_list_arg()` helper，用来从 MCP 参数里收集字符串数组
- `mempalace_onboarding` 支持：
  - `people`
  - `projects`
  - `aliases`
  - `wings`
  - `scan`
  - `auto_accept_detected`
- `mempalace_split` 支持：
  - `source_dir`
  - `output_dir`
  - `min_sessions`
  - `dry_run`
- `requires_existing_palace()` 里把这三条工具列入 no-palace 例外，因为它们本来就不是 palace 读写面
- `rust/tests/mcp_integration.rs` 补了这三条工具的成功路径回归
- `rust/README.md` 补了 MCP bootstrap/tooling 说明

## 关键知识

这次的关键不是“多加几个工具名”，而是**明确区分 palace 工具和 project-local 工具**。

例如：

- `mempalace_status`、`mempalace_search` 明显依赖 palace
- `mempalace_onboarding`、`mempalace_normalize`、`mempalace_split` 并不依赖已有 palace

所以 `requires_existing_palace()` 不能把所有新工具一刀切拦掉。  
如果不把这层边界分清，MCP host 会在完全合法的 bootstrap 场景下收到错误的 `No palace found`。

## 补充知识

### 1. MCP 最好直接吃结构化数组，而不是复刻 CLI 的字符串 flag

CLI 里的 `--person "Riley,daughter,personal"` 是终端友好的协议。  
MCP 里更自然的是：

```json
{
  "people": ["Riley,daughter,personal", "Ben,co-founder,work"]
}
```

这次虽然仍然复用了 `parse_person_arg()` 的字符串语义，但参数层已经允许 MCP 直接传数组，而不是逼 host 模拟多次 CLI flag。

### 2. `normalize` 最适合做成“单文件纯函数工具”

`normalize_conversation_file()` 这类能力没有 palace 依赖，也不应该偷偷落盘。  
做成 MCP 工具时，最稳的行为是：

- 输入一个文件
- 返回结构化 preview
- unsupported 时给 tool-level `error + hint`

不要顺手把它改成“自动写回文件”，否则和现有 CLI / 测试语义很快会分叉。

## 验证

实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

本次新增回归覆盖了：

- `tools/list` 里出现 `mempalace_onboarding`
- `tools/list` 里出现 `mempalace_normalize`
- `tools/list` 里出现 `mempalace_split`
- `mempalace_onboarding` 能生成项目本地 bootstrap 摘要
- `mempalace_normalize` 能返回 JSONL transcript 的规范化结果
- `mempalace_split` 能返回 transcript mega-file 的 split preview

## 未覆盖项

- 这次没有补 `mempalace_onboarding` / `normalize` / `split` 的专门错误路径回归，当前主要依赖统一的 tool-level `error + hint`
- 这次没有把 `hook` / `instructions` 接进 MCP
- 这次没有新增 Python 端对应 MCP 面，只扩了 Rust MCP surface
