# 背景

前面已经把 `repair --help` 的顶层语义锁进了 CLI 集成测试，但 `repair scan`、`repair prune`、`repair rebuild` 这三个真正执行面的帮助文本还没有单独测试。

这意味着只要顶层 help 还在，哪怕具体子命令的参数说明漂移了，当前测试也不一定会报警。为了把 maintenance 这一支 help 契约收完整，这一轮继续把 repair 子命令各自的帮助语义补锁进去。

# 主要目标

- 为 `repair scan --help` 增加 help 断言
- 为 `repair prune --help` 增加 help 断言
- 为 `repair rebuild --help` 增加 help 断言

# 改动概览

- 更新 `rust/tests/cli_integration.rs`
  - 新增 `cli_repair_scan_help_mentions_wing_filter`
    - 锁 `repair scan` 的 drift/corrupt_ids 语义
    - 锁 `--wing` 过滤说明
  - 新增 `cli_repair_prune_help_mentions_confirm_flag`
    - 锁 `repair prune` 的 queued IDs 删除语义
    - 锁 `--confirm` 说明
  - 新增 `cli_repair_rebuild_help_mentions_vector_rebuild`
    - 锁 `repair rebuild` 的 SQLite -> vector store 重建语义

# 关键知识

顶层 `repair --help` 和子命令 `repair scan|prune|rebuild --help` 是两层不同契约：

1. 顶层 help 告诉用户有哪些修复路径
2. 子命令 help 告诉用户每条路径具体会做什么

如果只测顶层，用户最关心的执行语义仍然可能悄悄漂移。

# 补充知识

对子命令 help 来说，最有价值的断言通常不是 flag 名本身，而是执行后果说明：

- `write corrupt_ids.txt`
- `Delete IDs listed in corrupt_ids.txt`
- `Rebuild the vector store from SQLite drawers`

这些句子直接对应真实操作风险，比只断言 `scan` / `prune` / `rebuild` 这样的命令名更稳。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_repair_scan_help_mentions_wing_filter -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_repair_prune_help_mentions_confirm_flag -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test cli_repair_rebuild_help_mentions_vector_rebuild -- --exact`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有修改 `rust/src/` 实现逻辑
- 这次没有继续改 `rust/README.md` 或 `docs/parity-ledger.md`
- `registry` 子命令族的 help 仍可作为下一轮一致性收口目标
