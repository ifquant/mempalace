# 背景

前面已经把 `mine`、`search`、`repair`、`registry` 这些高频命令的帮助文本覆盖补得比较完整了，但 `onboarding`、`split`、`normalize` 这几条 project-facing 命令还存在一层薄弱点：测试更偏向 flag 是否存在，没有把关键位置参数和执行语义完整锁住。

这类命令恰好又是新用户最容易直接照着 help 运行的入口，所以继续沿着同一条 “README/help/tests 一致性” 主线，把它们的剩余帮助语义补齐。

# 主要目标

- 扩充 `onboarding --help` 的位置参数和 mode/wings 语义覆盖
- 扩充 `split --help` 的输入/输出/dry-run 语义覆盖
- 扩充 `normalize --help` 的 human 预览语义覆盖

# 改动概览

- 更新 `rust/tests/cli_integration.rs`
  - `cli_onboarding_help_mentions_mode_people_and_scan`
    - 补锁命令简介
    - 补锁项目目录位置参数说明
    - 补锁 mode 语义说明
    - 补锁 wings 说明
  - `cli_split_help_mentions_transcript_megafiles`
    - 补锁输入目录说明
    - 补锁输出目录说明
    - 补锁 dry-run 说明
  - `cli_normalize_help_mentions_chat_export_normalization`
    - 补锁 human 预览说明

# 关键知识

对 project-facing 命令来说，help 文案最容易缺的不是 flag 名，而是“这条命令到底拿什么当输入、会产出什么、dry-run 会不会写文件”这类执行语义。

所以这类测试的优先级应当是：

1. 命令简介
2. 位置参数说明
3. 会影响行为理解的 flag 语义

# 补充知识

`split` 和 `normalize` 都是用户很可能直接用导出文件去跑的命令。  
如果 help 里没有把这些语义锁住：

- `split` 的输入目录 / 输出目录 / dry-run
- `normalize` 的 preview 行为

后续 README 和 CLI 帮助就容易慢慢偏离，而测试不会第一时间报警。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_onboarding_help_mentions_mode_people_and_scan -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_split_help_mentions_transcript_megafiles -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_normalize_help_mentions_chat_export_normalization -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有修改 `rust/src/` 实现代码
- 这次没有继续改 `rust/README.md` 或 `docs/parity-ledger.md`
- 如果继续同一条线，下一轮更适合开始收 README/help/tests 的最终残项，而不是继续机械加 help 测试
