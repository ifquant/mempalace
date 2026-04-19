# 背景

前两轮已经把一批高频读面和运维命令的帮助文本锁进了 CLI 集成测试，但 `hook`、`recall`、`layers-status` 这组 helper/read 命令的覆盖还偏浅，更多只是确认命令存在，没有把关键参数语义锁住。

这类命令虽然实现已经稳定，但一旦 clap 文案漂移，README、help 输出和测试之间就会重新失配。因此继续沿着同一条“一致性收口”主线，把这组命令的帮助语义补实。

# 主要目标

- 扩充 `hook --help` 的子命令语义覆盖
- 新增 `hook run --help` 的参数语义覆盖
- 扩充 `recall --help` 的 wing/room/results/human 语义覆盖
- 扩充 `layers-status --help` 的 human 输出语义覆盖

# 改动概览

- 更新 `rust/tests/cli_integration.rs`
  - `cli_hook_help_mentions_stdio_behavior`
    - 补锁 `run` 子命令的语义描述
  - 新增 `cli_hook_run_help_mentions_hook_name_and_harness`
    - 锁 `hook run --help`
    - 补锁 `--hook` 和 `--harness` 的参数说明
  - `cli_recall_help_mentions_wing_room_and_results`
    - 补锁 wing 过滤说明
    - 补锁 room 过滤说明
    - 补锁 results 上限说明
    - 补锁 human 输出说明
  - `cli_layers_status_help_mentions_layer_stack`
    - 补锁 human 输出说明

# 关键知识

`hook --help` 和 `hook run --help` 是两层不同的契约：

1. 顶层 `hook --help` 说明这组命令是 stdin/stdout JSON 协议面
2. `hook run --help` 说明具体执行面要传什么参数

如果只测顶层 help，就很容易漏掉 `run` 子命令的参数文案漂移。

# 补充知识

对 `recall` 这类读面命令来说，只断言 `--wing` / `--room` / `--results` 这些 flag 名不够稳。  
更重要的是锁住对应说明句子，比如：

- `Limit recall to one project/wing`
- `Limit recall to one room`
- `Maximum number of drawers to return`

这样以后即使 flag 还在，但帮助语义变得含糊，测试也会及时失败。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_hook_help_mentions_stdio_behavior -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_hook_run_help_mentions_hook_name_and_harness -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_recall_help_mentions_wing_room_and_results -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_layers_status_help_mentions_layer_stack -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有修改 `rust/src/` 里的 clap schema
- 这次没有继续改 `rust/README.md` 或 `docs/parity-ledger.md`
- 其余 help/test 一致性尾项仍可继续按相同方式补锁
