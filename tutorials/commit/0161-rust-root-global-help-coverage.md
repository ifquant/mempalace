# 背景

前几轮已经把大部分子命令 help 的关键语义锁进了 CLI 集成测试，但顶层 root help 里最容易影响实际使用的两类信息还没有被单独锁住：

- 全局选项：`--palace`、`--hf-endpoint`
- `mcp --help` 里的“setup vs serve”入口语义

这类文案一旦漂移，用户会直接失去最关键的入口提示，但如果测试只盯命令列表和示例命令，就不一定能及时发现。

# 主要目标

- 扩充 root `--help` 的全局参数语义覆盖
- 补锁 `mcp --help` 的 read-only server 语义

# 改动概览

- 更新 `rust/tests/cli_integration.rs`
  - `cli_root_help_mentions_core_commands_and_examples`
    - 补锁 `--palace <PALACE>`
    - 补锁 palace 路径说明
    - 补锁 `--hf-endpoint <HF_ENDPOINT>`
    - 补锁 HuggingFace endpoint 覆盖说明
  - `cli_mcp_help_mentions_setup_and_serve_flags`
    - 补锁 `read-only MCP server` 语义

# 关键知识

root `--help` 不只是命令目录页，它还是最重要的“入口契约页”。  
对于第一次使用 Rust 版 MemPalace 的用户，顶层全局选项经常比单个子命令参数更关键，因为它决定：

1. palace 指向哪里
2. embedding 下载走哪个镜像或 endpoint

如果这两条语义不被测试锁住，后续 README、help、实际使用说明就很容易重新失配。

# 补充知识

`mcp --help` 的关键不只是 `--setup` 和 `--serve` 两个 flag 是否还在，更重要的是“这个命令本质上是在跑一个 read-only MCP server”这件事要持续明确。

因此这类测试要优先锁：

- 入口类型
- 运行方式
- 用户能否从 help 里直接理解这是 setup 路径还是 server 路径

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_root_help_mentions_core_commands_and_examples -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_mcp_help_mentions_setup_and_serve_flags -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有修改 `rust/src/` 实现代码
- 这次没有继续改 `rust/README.md` 或 `docs/parity-ledger.md`
- 其余 help/test 一致性尾项仍可继续按相同方式收口
