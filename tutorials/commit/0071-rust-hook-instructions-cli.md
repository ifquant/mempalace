# 背景

Rust 版 `mempalace` 之前已经补上了：

- project / convos mining
- search / status / repair / migrate
- AAAK `compress`
- `wake-up`

但 Python CLI 里还有两块正式表面没有迁过来：

1. `hook run`
2. `instructions <name>`

这两块看起来不像核心存储能力，但它们对真实 agent 集成很关键：

- `hook run` 负责 stop / precompact 自动保存节奏
- `instructions` 负责给上层 agent 输出稳定的技能说明文本

所以这一提交的目标，是把这两个 CLI 表面补成真正可运行、可测试、可复用的 Rust 能力，而不是留在 README 里当“以后再说”。

# 主要目标

- 给 Rust 新增 hook 模块，支持：
  - `session-start`
  - `stop`
  - `precompact`
- 保持 Python 风格的 stdin JSON -> stdout JSON 协议
- 给 Rust 新增 `instructions` 输出面
- 补齐 CLI 帮助、README、回归测试

# 改动概览

- 新增 [rust/src/hook.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/hook.rs)
  - 读取 stdin JSON
  - 解析 `session_id / stop_hook_active / transcript_path`
  - 统计 transcript 里的 human messages
  - 输出 `{}` 或 `{"decision":"block","reason":"..."}` 这类 hook JSON
  - 支持：
    - `session-start`
    - `stop`
    - `precompact`
  - 支持 Claude 风格 transcript 和 Codex 风格 transcript
- 新增 [rust/src/instructions.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/instructions.rs)
  - 按名称输出内置 markdown 指令
- 新增内置指令文件：
  - [rust/instructions/help.md](/Users/dev/workspace2/agents_research/mempalace/rust/instructions/help.md)
  - [rust/instructions/init.md](/Users/dev/workspace2/agents_research/mempalace/rust/instructions/init.md)
  - [rust/instructions/mine.md](/Users/dev/workspace2/agents_research/mempalace/rust/instructions/mine.md)
  - [rust/instructions/search.md](/Users/dev/workspace2/agents_research/mempalace/rust/instructions/search.md)
  - [rust/instructions/status.md](/Users/dev/workspace2/agents_research/mempalace/rust/instructions/status.md)
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `hook` 和 `instructions`
- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - 新增：
    - `hook run --hook ... --harness ...`
    - `instructions <name>`
- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - 新增 hook / instructions CLI 回归
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 把 hook / instructions 写成当前 Rust 能力事实

# 关键知识

## 1. Hook 的关键不是“命令存在”，而是 stdin/stdout 协议稳定

这类命令最容易做成假实现：

- 有个 CLI 名称
- 但真实 harness 接上时才发现 JSON shape 不对

这次 Rust 实现里最重要的其实不是 subcommand 本身，而是：

- 从 stdin 读 JSON
- 解析已知字段
- 再把结果稳定写回 stdout

也就是说，真正的兼容点是“协议面”，不是命令面。

## 2. `stop` hook 的核心状态不是 transcript，而是“上次保存点”

Python 逻辑里，`stop` 不是简单地“每当 transcript 里有 15 条消息就 block”，而是：

- 读取累计 human message 数
- 再减去 `last_save`
- 只在“距离上次保存又积累了 15 条”时 block

Rust 这里也保留了这个模式，只是把状态目录改成了 palace-local：

- Python：`~/.mempalace/hook_state`
- Rust：`<palace>/hook_state`

这能保证同一个 palace 的自动保存节奏是局部可迁移的。

## 3. `instructions` 最好直接输出静态 markdown，不要动态拼

这类文本看起来简单，但如果你把它做成运行时拼接：

- 很容易因为格式微调导致上层 agent prompt 漂移
- 也不利于 review 和 diff

所以这次直接把内容做成文件：

- 更接近 Python 的 `instructions/*.md`
- 更容易审查
- 更适合后续继续收口和补文案

# 补充知识

## 1. “局部状态路径”是 local-first 迁移里一个经常被忽视的设计点

很多人会注意数据库路径，却忽略这种“会话控制小状态”：

- hook last-save marker
- hook log
- 临时集成状态

但这些东西一旦留在全局 home 路径里，就会让：

- 测试隔离变差
- 多 palace 并行时互相污染
- 备份/迁移不完整

所以这次把 `hook_state` 放到 `<palace>/hook_state/`，是一个很小但很实用的 local-first 收口动作。

## 2. Transcript 兼容测试要覆盖“跳过 command-message”

如果只数 `role=user`，会把很多 harness 的控制消息也算进来，结果是：

- stop hook 提前触发
- 自动保存节奏错乱

所以测试里显式覆盖了：

- Claude 风格 `message.role=user`
- Codex 风格 `event_msg.user_message`
- `<command-message>` 过滤

这类看起来很细，但一旦漏掉，实际用起来会非常烦。

# 验证

本次实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

新增覆盖包括：

- `cli_hook_help_mentions_stdio_behavior`
- `cli_instructions_help_outputs_markdown`
- `cli_hook_session_start_outputs_empty_json_and_initializes_state`
- `cli_hook_stop_blocks_after_15_messages`
- `cli_hook_stop_passes_through_when_already_active`
- `cli_hook_precompact_always_blocks`
- `hook::tests::count_human_messages_skips_command_messages`

# 未覆盖项

- 还没迁 Python 的 shell hook 脚本本身，只迁了 Rust 内部 hook 逻辑和 CLI surface。
- 还没做 `instructions` 的动态生成，只做了静态 markdown 对应面。
- 还没把 hook / instructions 接进 MCP；这轮只做 CLI 能力。
- Rust hook 的状态目录是 `<palace>/hook_state/`，没有复刻 Python 的全局 `~/.mempalace/hook_state/` 路径。
