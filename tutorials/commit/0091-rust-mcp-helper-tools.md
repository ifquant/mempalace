## 背景

Rust 版前面已经把大部分业务型 MCP surface 补到了位：

- search / taxonomy / KG
- maintenance / AAAK
- onboarding / normalize / split
- registry

但还有两类“辅助型能力”只存在于 CLI：

- `instructions <name>`：输出内建操作说明
- `hook run ...`：给 harness 侧 auto-save / precompact 逻辑复用

如果 MCP 没有这两个入口，host 想通过 MCP 统一调用 Rust helper surface 时，就只能绕回 CLI，行为面会被割裂。

## 主要目标

- 给 Rust MCP 新增 `mempalace_instructions`
- 给 Rust MCP 新增 `mempalace_hook_run`
- 保持这两个工具不依赖现有 palace
- 给它们补成功路径和失败路径回归
- 把这块能力同步写进 README

## 改动概览

- 更新 `rust/src/hook.rs`
  - 新增 `run_hook_with_data()`，把原来只能从 stdin 读取的 hook 逻辑拆成可复用函数
- 更新 `rust/src/mcp.rs`
  - 新增 `mempalace_instructions`
  - 新增 `mempalace_hook_run`
  - 把 `stop_hook_active` 加入参数 coercion
  - 把这两个工具纳入 no-palace 白名单
- 更新 `rust/tests/mcp_integration.rs`
  - 补 helper tools 的成功路径
  - 补缺参 / 坏输入的 tool-level error 回归
- 更新 `rust/README.md`

## 关键知识

### 1. CLI helper 想复用到 MCP，最好先拆“纯函数入口”

这次 `hook run` 原本的问题不是功能没有，而是入口太 CLI：

- 从 stdin 读 JSON
- 直接在 CLI 路径里解析

MCP 不适合再去模拟 stdin。更稳的做法是先拆：

- `run_hook()` 保留原 CLI 行为
- `run_hook_with_data()` 提供结构化输入入口

这样 CLI 和 MCP 共用同一套核心逻辑，后面如果再接别的 surface，也不会再复制一份 hook 分支。

### 2. “辅助工具”也要保持 MCP 的统一失败协议

这次新增的是 helper tools，不是核心搜索工具，但失败时也必须继续遵守 MCP 当前约定：

- 返回 `result.content[0].text`
- 里面是结构化 JSON
- 带 `error`
- 带 `hint`

否则 host 侧一旦把 helper tool 和业务 tool 混着调，就会遇到一半是 tool-level error、一半是 transport error 的混乱状态。

## 补充知识

### 1. 能不依赖 palace 的工具，最好显式放进 no-palace 白名单

`instructions` 和 `hook_run` 本质都不需要先有 drawer 数据。  
如果忘了把它们放进 `requires_existing_palace()` 的豁免列表，MCP host 会先被 “No palace found” 挡住，根本走不到真正逻辑。

### 2. 参数 coercion 是 MCP host 兼容性的常见坑

很多 host 不一定会把布尔和整数严格按 JSON 类型传过来，经常会出现：

- `"false"`
- `"3"`

这次 `mempalace_hook_run.stop_hook_active` 也补进了 coercion，目的就是避免 host 传字符串时行为漂移。  
这类兼容层应该集中做，不要散落在业务逻辑里临时判断。

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

- `mempalace_instructions` 成功返回内建 markdown
- `mempalace_hook_run` 成功运行 `stop` hook，并返回 block decision
- helper tools 缺参时返回稳定 `error + hint`
- `hook_run` 在坏 harness 输入下返回稳定 tool-level 错误

## 未覆盖项

- 这次没有把 `instructions` / `hook_run` 再额外做一层 CLI 行为变更，只补 MCP surface
- 这次没有扩更多 instruction 名称，仍然沿用当前 Rust 已支持的那几份
- 这次没有改 Python 端 helper surface，只收紧 Rust MCP 对齐面
