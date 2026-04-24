## 背景

前两轮已经把很多新 MCP 工具接进了 Rust：

- maintenance / AAAK：
  - `repair`
  - `repair_scan`
  - `repair_prune`
  - `repair_rebuild`
  - `compress`
  - `dedup`
- project bootstrap / transcript prep：
  - `onboarding`
  - `normalize`
  - `split`

但当时的回归主要锁的是“成功路径能跑通”，失败路径还不够扎实。对 MCP 来说，这类工具如果一出错就漏成 transport error，会直接让 host 侧体验断裂。

## 主要目标

- 给 maintenance MCP tools 补 broken-sqlite 的统一错误回归
- 给 bootstrap MCP tools 补缺参错误回归
- 给 bootstrap MCP tools 补坏输入错误回归
- 确认这些工具继续保持 `tool-level error + hint`，而不是抬成 MCP transport error

## 改动概览

- 更新 `rust/tests/mcp_integration.rs`
- 新增：
  - `mcp_maintenance_tools_return_tool_level_error_payloads_on_broken_sqlite`
  - `mcp_project_bootstrap_tools_return_tool_level_errors_for_missing_args`
  - `mcp_project_bootstrap_tools_return_tool_level_errors_for_bad_inputs`
- 更新 `rust/README.md`，把这批新工具的错误语义写成仓库事实

## 关键知识

### 1. MCP “失败路径一致性”比单个错误文案更重要

这次真正锁住的不是某个具体 SQLite 错误字符串，而是这几个事实：

- 不抬成 transport error
- `result.content[0].text` 里能解析出结构化 JSON
- JSON 里有：
  - `error`
  - `hint`

这样 host 侧才能稳定处理，而不会因为底层错误源头不同就整条协议断掉。

### 2. maintenance 和 bootstrap 的失败类型不同，测试也应该分开

这两组工具失败原因完全不同：

- maintenance 更像：
  - broken sqlite
  - drift
  - palace state 不一致
- bootstrap 更像：
  - 缺参数
  - 输入格式错
  - transcript 不支持

如果把它们混成一个大测试，后面排查失败时会很痛。分开后，一眼就能知道是“数据面坏了”还是“参数面坏了”。

## 补充知识

### 1. 测试 broken-sqlite 时，不需要每个工具都造完整 fixture

最省成本的做法就是：

1. `app.init()`
2. 直接把 `palace.sqlite3` 覆写成垃圾内容
3. 调工具

这样能稳定逼出 SQLite 打开失败，而且不需要为每个工具重新造业务数据。

### 2. 对 transcript normalize 的坏输入，最稳的是用 unsupported file，而不是硬凑半残 JSON

半残 JSON 可能会随着 parser 细节变化，今天报这个错，明天报那个错。  
直接给一个 `.bin` 文件，测试目标更明确：

- 不是支持的 transcript/export
- 应该返回 normalize 的 tool-level 错误

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

- maintenance MCP tools 在 broken-sqlite 下继续返回 `error + hint`
- onboarding / normalize / split 缺参时返回稳定 tool-level payload
- onboarding 的坏 `people` 输入会返回明确格式提示
- normalize 的 unsupported file 会返回明确 transcript hint

## 未覆盖项

- 这次没有补 maintenance MCP tools 的 no-palace 场景，因为它们当前已经依赖统一 `No palace found` gate
- 这次没有补 `split` 的坏目录路径专项回归
- 这次没有新增 Python 端测试，只收紧 Rust MCP surface
