# 背景

前几轮连续补的是 Rust CLI help 的集成测试覆盖。到这一轮，root/global flags、helper/read、maintenance、registry、project-facing 命令的帮助契约已经大范围被锁进了 `cli_integration.rs`。

如果 README 和 parity ledger 还继续把这块笼统写成 “help/test consistency 仍是主要剩余工作”，就会失真：真正还没收完的已经不再是 help 契约本身，而是 README 示例和更深的非 CLI 行为审计。

# 主要目标

- 更新 `rust/README.md`，让当前 parity 状态描述和最新 help/test 覆盖一致
- 更新 `docs/parity-ledger.md`，把剩余项从笼统的 help/test 审计收窄为 README/example drift

# 改动概览

- 更新 `rust/README.md`
  - 新增一条当前状态：
    - CLI integration 已经覆盖 root/global flags、helper/read、maintenance、registry、project-facing commands
  - 将 remaining work 从：
    - `truth-in-docs, help/test consistency, and deeper non-CLI behavior audits`
    调整为：
    - `README/example truth-in-docs and deeper non-CLI behavior audits`
- 更新 `docs/parity-ledger.md`
  - 在 Snapshot 里补记：
    - Rust CLI help coverage 已经被较广范围的 integration tests 锁住
  - 将 Remaining Work 第一项从：
    - `README/help/test consistency audit`
    调整为：
    - `README/example consistency audit`

# 关键知识

当一条工作线连续做了很多 “锁 help 契约” 的测试补丁之后，文档里的“remaining”描述要同步收窄。  
否则后续阅读 README 或 parity ledger 的人，会误以为 help 契约还是大面积未审状态，从而低估当前收口进度。

# 补充知识

Parity ledger 不是 backlog 垃圾桶。  
它更像一张“当前用户可见真相表”：

1. 已经对齐的写清楚
2. 有意偏离的写清楚
3. 真正剩余的范围要尽量窄、尽量具体

所以当 `help/test consistency` 已经被多轮切片大幅吃掉时，继续保留一个过宽的 remaining 标签，本身就是一种 truth-in-docs 漂移。

# 验证

- 交叉检查 `rust/README.md` 与 `docs/parity-ledger.md` 的 parity 描述是否一致
- 交叉检查最近已落下的 help 覆盖切片是否足以支撑 “CLI help contracts broadly locked” 这一表述
- 本次是文档 truth pass，没有新增 Rust 运行时代码改动

# 未覆盖项

- 这次没有修改 `rust/src/` 或 `rust/tests/`
- 这次没有继续补新的 CLI help 测试
- 更深的非 CLI 行为 parity 审计仍然保留在 ledger 的 `remaining` 里
