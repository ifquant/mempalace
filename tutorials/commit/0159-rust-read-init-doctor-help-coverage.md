# 背景

上一轮已经把 `status`、`dedup`、`prepare-embedding` 这组 help 文案锁进了 CLI 集成测试，但 `compress`、`wake-up`、`init`、`doctor` 这几条高频入口的覆盖还偏薄，仍然只校验了命令标题和 `--human` 一类浅层文案。

如果后续有人改了 clap 参数描述、删掉了关键 flag 说明，现有测试不一定能及时报警。这个切片继续沿着 “README/help/tests 一致性” 主线，把这组命令的关键参数语义补锁一层。

# 主要目标

- 扩充 `compress --help` 的参数语义覆盖
- 扩充 `wake-up --help` 的 wing 过滤语义覆盖
- 扩充 `init --help` 的位置参数和 `--yes` 语义覆盖
- 扩充 `doctor --help` 的 warm-up 语义覆盖

# 改动概览

- 更新 `rust/tests/cli_integration.rs`
  - `cli_compress_help_mentions_human_output`
    - 补锁 wing 限定语义
    - 补锁 dry-run 预览语义
  - `cli_wake_up_help_mentions_human_output`
    - 补锁按 wing 显示 wake-up 上下文的说明
  - `cli_init_help_mentions_human_output`
    - 补锁项目目录位置参数描述
    - 补锁 `--yes` 的自动接受 bootstrap 语义
  - `cli_doctor_help_mentions_human_output`
    - 补锁 warm embedding 语义

# 关键知识

`--help` 覆盖不是只看命令名是否存在，更重要的是锁住“用户为什么要传这个参数”。  
例如 `--yes`、`--dry-run`、`--wing` 这种 flag，如果只断言 flag 名本身，未来即使帮助文案被改成含糊甚至误导性的描述，测试也不会失败。

所以这类 help 测试更稳的写法是：

1. 锁命令标题
2. 锁关键位置参数描述
3. 锁最重要的 flag 语义句子

# 补充知识

clap 的帮助文本会天然包含 flag 名，比如 `--wing`、`--dry-run`。  
但真正容易漂移的是 `help = "..."`
 里的自然语言说明，因此断言完整语义短句，比只断言 flag 名更有价值。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_compress_help_mentions_human_output -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_wake_up_help_mentions_human_output -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_init_help_mentions_human_output -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_doctor_help_mentions_human_output -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有改 `rust/src/` 里的 clap schema，只补测试覆盖
- 这次没有继续碰 `README` 或 `docs/parity-ledger.md`
- 其余 help/test 一致性残项仍可继续沿相同方式补锁
