# 背景

前面已经把 `registry --help` 的顶层命令目录锁进了 CLI 集成测试，但 `summary`、`lookup`、`learn`、`add-person`、`add-project`、`add-alias`、`query`、`research`、`confirm` 这些真正使用的子命令还没有单独覆盖。

这会留下一个明显空档：只要顶层目录还在，哪怕具体子命令的位置参数、上下文参数、human 输出说明发生漂移，当前测试也不一定能发现。

# 主要目标

- 为 registry read 子命令补 help 覆盖
- 为 registry write 子命令补 help 覆盖
- 为 registry research 子命令补 help 覆盖

# 改动概览

- 更新 `rust/tests/cli_integration.rs`
  - 新增 `cli_registry_read_subcommands_help_cover_paths_and_human_output`
    - 覆盖 `summary`
    - 覆盖 `lookup`
    - 覆盖 `learn`
    - 覆盖 `query`
  - 新增 `cli_registry_write_subcommands_help_cover_entity_fields`
    - 覆盖 `add-person`
    - 覆盖 `add-project`
    - 覆盖 `add-alias`
  - 新增 `cli_registry_research_subcommands_help_cover_confirmation_fields`
    - 覆盖 `research`
    - 覆盖 `confirm`

# 关键知识

`registry` 这一族命令的 help 契约可以分成三层：

1. read：看 registry 里已有的信息
2. write：直接写入实体和别名
3. research：先缓存、再确认推广

如果不按这三层分别锁 help，很容易出现顶层目录不变，但某一类子命令的关键参数文案漂移而无人察觉。

# 补充知识

对 `lookup`、`confirm` 这种命令，最重要的不是命令名本身，而是那些会影响理解和使用方式的字段说明：

- ambiguous-name 的 `context`
- confirm 时的 `entity type`
- confirm 时的 `relationship`
- confirm 时的 `context bucket`

这些字段说明一旦变得含糊，用户就很难从 help 里理解 registry 的实际工作流。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_registry_read_subcommands_help_cover_paths_and_human_output -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_registry_write_subcommands_help_cover_entity_fields -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_registry_research_subcommands_help_cover_confirmation_fields -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有修改 `rust/src/` 实现逻辑
- 这次没有继续改 `rust/README.md` 或 `docs/parity-ledger.md`
- 若继续同一条线，下一轮可以转去 `project` 子命令族或开始收 README/help/tests 的最终残项
