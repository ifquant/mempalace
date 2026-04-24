## 背景

前一轮已经把 Rust CLI 里的 `wake-up`、`recall`、`layers-status` 补出来了，但 MCP 侧还看不到这组 layer 能力。这样会留下一个明显断层：终端里能调用 Layer 0-3，MCP host 却只能看到 `search` 和 `status` 这些旧面。

这次的目标很窄，就是把已有的 layer service 暴露到 Rust MCP，避免再在 service 之外复制一套拼装逻辑。

## 主要目标

- 给 Rust MCP 新增 `mempalace_wake_up`
- 给 Rust MCP 新增 `mempalace_recall`
- 给 Rust MCP 新增 `mempalace_layers_status`
- 让这三个工具直接复用现有 service 返回结构
- 补齐 MCP integration test 和 README 说明

## 改动概览

- 在 `rust/src/mcp.rs` 的 `tools()` 里新增三条工具 schema
- 在 `call_tool()` 里新增三条 dispatcher 分支：
  - `mempalace_wake_up`
  - `mempalace_recall`
  - `mempalace_layers_status`
- `mempalace_recall` 新增 `limit` 参数收敛，兼容 MCP 里常见的字符串数字输入
- 在 `rust/tests/mcp_integration.rs` 里把三条工具都纳入现有 `mcp_read_tools_work()` 主回归
- 在 `rust/README.md` 里把 layer MCP 面写成仓库事实

## 关键知识

Rust 这次没有新建任何 layer-only model。

原因是 service 层已经有：

- `App::wake_up()`
- `App::recall()`
- `App::layer_status()`

而 model 层也已经有：

- `WakeUpSummary`
- `RecallSummary`
- `LayerStatusSummary`

所以 MCP 最稳的做法不是重新拼一个“更像 MCP”的私有 JSON，而是直接 `serde_json::to_value(summary)`。这样有两个好处：

1. CLI / service / MCP 共用同一份结构，减少 drift
2. 以后 layer 输出再扩字段时，MCP 不容易漏同步

## 补充知识

### 1. `requires_existing_palace()` 不一定每次都要改

这个函数的逻辑是“默认需要已有 palace，只有少数写工具例外”。所以新增读工具时，很多时候不需要把名字再加进去，因为默认已经会走 no-palace gate。

### 2. MCP 参数收敛最好放在统一入口

像 `limit: "2"` 这种输入很常见。如果在每个工具分支里手动 `parse()`，很快就会散掉。把 coercion 集中放在 `coerce_argument_types()`，后面排查 MCP 参数问题会轻很多。

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

- `tools/list` 里出现 `mempalace_wake_up`
- `tools/list` 里出现 `mempalace_recall`
- `tools/list` 里出现 `mempalace_layers_status`
- `mempalace_wake_up` 成功返回 `kind/identity/layer1`
- `mempalace_recall` 成功返回 `kind/results/total_matches`
- `mempalace_layers_status` 成功返回 layer 说明字段

## 未覆盖项

- 这次没有新增 layer MCP 的 broken-sqlite 专项错误回归；当前仍依赖已有工具级 `error + hint` 模式
- 这次没有把 layer trio 接到 Python 端，只改了 Rust MCP
- 这次没有新增 layer 相关的 MCP 文本人类格式，因为 MCP 目前统一走结构化 JSON content
