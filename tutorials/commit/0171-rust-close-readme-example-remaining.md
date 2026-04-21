# 背景

前几轮连续推进了 README/example consistency：

- 补齐 verification command 示例
- 补齐 helper / AAAK / hook 示例
- 补齐 `init` / `mine` 主链示例
- 收紧 README 的阶段口吻和 MCP framing

此时 parity ledger 里继续把 `README/example consistency audit` 放在 `remaining`，已经不再准确。它会让后续读者误以为 README/example 仍然是一个主要未收口区域。

# 主要目标

- 在 README 中写明当前示例集合已经覆盖主要流程
- 在 parity ledger 中关闭 `README/example consistency audit` 这个 remaining 项
- 保留真正还需要后续推进的 deeper behavior audit

# 改动概览

- 更新 `rust/README.md`
  - 增加 “README verification examples now cover the main project, palace, registry, helper, MCP, and embedding flows”
  - 将 remaining work 收窄为：
    - deeper non-CLI behavior audits
    - future residual parity batches
- 更新 `docs/parity-ledger.md`
  - Snapshot 增加 README 示例覆盖现状
  - 从 Remaining Work 表中移除 `README/example consistency audit`

# 关键知识

Parity ledger 的 remaining 表不应该长期保留已经实际收掉的项。  
否则它会从“事实账”退化成“历史 TODO 残影”。

这次不是说 README 永远不会再漂移，而是说当前已确认的 README/example audit 已经完成到足以不再作为主要 remaining 项保留。

# 补充知识

关闭一个 remaining 项时，需要同时保留后续安全阀：

- 如果未来发现新用户可见差距，再先写回 ledger
- 如果是更深行为差异，则进入 `Deeper non-CLI behavior audit`

这样既不会虚假宣称所有事情都完成，也不会让已完成的文档一致性工作继续占用 remaining 列表。

# 验证

- 交叉检查 `rust/README.md` 与 `docs/parity-ledger.md` 的 remaining 表述是否一致
- 交叉检查 README verification examples 是否已经覆盖主要 project / palace / registry / helper / MCP / embedding flows
- 本次是文档 truth pass，没有新增 Rust 运行时代码改动

# 未覆盖项

- 这次没有修改 `rust/src/`
- 这次没有修改 `rust/tests/`
- 更深的非 CLI 行为 parity 审计仍然保留在 ledger 的 `remaining` 里
