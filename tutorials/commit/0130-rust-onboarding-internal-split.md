# 背景

前几轮已经把 bootstrap、miner、convo、registry 等大模块逐步拆成更清楚的边界，但 `rust/src/onboarding.rs` 还同时承担三种不同职责：

- onboarding orchestration
- interactive prompt / terminal UI
- request normalization、dedupe、auto-detected merge、CLI/MCP 共享 parse helper

这会让后面如果只想改交互提问文案，也必须碰到 onboarding 主流程；反过来如果只是改 merge/dedupe 规则，也会把一大坨 prompt 细节一起带进 diff。

# 主要目标

把 Rust onboarding 内部继续按职责切开，同时保持外部 API 不变：

- `run_onboarding()` 入口不变
- `OnboardingRequest` 结构不变
- `parse_person_arg()` / `parse_alias_arg()` / `ambiguous_names()` 继续从 `crate::onboarding` 对外可用
- CLI、MCP、runtime 调用方不需要改 import 路径

# 改动概览

这次新增了两个内部模块：

- `rust/src/onboarding_prompt.rs`
- `rust/src/onboarding_support.rs`

并把 `rust/src/onboarding.rs` 收成 orchestration 层。

## 1. `onboarding_prompt`

这里现在承接：

- `prompt_for_request()`
- `ask_yes_no()`
- `prompt_mode()`
- `prompt_people()`
- `prompt_projects()`
- `prompt_wings()`
- terminal UI helper（header / rule / prompt）

也就是 onboarding 里“和终端交互”那部分。

## 2. `onboarding_support`

这里现在承接：

- mode normalization
- 默认 wings / 默认 person context
- people / project / alias dedupe
- auto-detected people / project merge
- `split_name_relationship()`
- `parse_person_arg()` / `parse_alias_arg()` / `ambiguous_names()`

也就是 onboarding 里“和数据整形、共享 helper、输入解析”相关的那部分。

## 3. `onboarding`

这里现在只保留：

- `OnboardingRequest`
- `run_onboarding()`
- 对 parse helper 的 re-export
- onboarding summary 组装

这样 `onboarding.rs` 的重心重新回到“如何组织 onboarding 过程”，而不是继续夹带所有 prompt 和 helper 细节。

# 关键知识

## 1. 交互式 prompt 和非交互流程的变化节奏不同

这两层虽然都属于 onboarding，但维护节奏不一样：

- prompt 层更容易因为文案、默认值、交互顺序而调整
- support 层更容易因为 dedupe、normalize、merge 规则而调整
- orchestration 层更关心什么时候 scan、什么时候写文件、最后 summary 怎么组装

把它们混在一个文件里，任何一类小改动都会制造跨职责的大 diff。拆开之后，边界更清楚。

## 2. 对外 re-export 能减少上层抖动

这次 `parse_person_arg()`、`parse_alias_arg()`、`ambiguous_names()` 的实现已经挪到了 `onboarding_support`，但 `project_cli_bootstrap` 和 `mcp_runtime_project` 仍然继续从 `crate::onboarding` 引用它们。

这里的做法是：

- 真正实现下沉到 support 模块
- `onboarding.rs` 继续 `pub use` 对外暴露

这样既能收紧内部结构，也不会把 CLI / MCP / runtime 的 import 路径一起搅动。

# 补充知识

## 为什么 `merge_detected_*` 改成接收 confirm closure

原先 merge helper 直接依赖 `ask_yes_no()`，这会让 support 层反向依赖 prompt 层，边界重新糊掉。

所以这次改成：

- support 层只关心“是否确认”
- prompt 层把 `ask_yes_no()` 作为 closure 传进去

这样 merge 逻辑仍然能复用交互确认，但 support 模块不会再次绑死到终端 UI。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

验证通过，说明这次 onboarding 内部分层没有改变现有 CLI / MCP / runtime 的外部行为。

# 未覆盖项

这次没有继续改：

- `project_cli_bootstrap`
- `mcp_runtime_project`
- `init_runtime`

因为目标只是把 `onboarding.rs` 的内部职责拆开，而不是继续往更高层改命令分发或 runtime 结构。
