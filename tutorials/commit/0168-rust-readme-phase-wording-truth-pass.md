# 背景

前几轮已经持续把 README 的 parity 状态、命令示例和 remaining work 收得更贴近当前事实，但 README 里还残留了一类更隐蔽的旧口吻：章节标题仍然把当前状态写成 “first-phase”。

在当前进度下，这种说法已经不够准确。Rust 侧现在的状态更像“主能力面已成型、剩余工作主要是 truth-in-docs 和更深行为审计”，而不是一个刚起步的第一阶段能力清单。

# 主要目标

- 把 README 中仍然带有早期阶段感的章节标题改成更准确的当前表述

# 改动概览

- 更新 `rust/README.md`
  - 将 `Current first-phase support` 改为 `Current user-visible support`
  - 将 `Intentionally not in this first Rust phase` 改为 `Still intentionally out of scope for the current Rust phase`

# 关键知识

文档 truth pass 不只是修命令示例，也包括修“语气”和“阶段判断”。  
当实现已经跨过“第一阶段能力清单”的状态时，继续保留这种标题，会误导读者低估当前覆盖面。

# 补充知识

这类标题级修正通常比正文更重要，因为很多读者只会扫标题和分段开头。  
如果标题还停留在旧阶段判断，哪怕正文内容已经更新，整体感知仍然会偏旧。

# 验证

- 交叉检查 `rust/README.md` 新标题是否和当前 parity 状态描述一致
- 确认这次只收文档口径，没有引入新的命令或行为声明
- 本次是文档 truth pass，没有新增 Rust 运行时代码改动

# 未覆盖项

- 这次没有修改 `rust/src/`
- 这次没有修改 `rust/tests/`
- 这次没有修改 `docs/parity-ledger.md`
- 更深的非 CLI 行为 parity 审计仍然保留在 ledger 的 `remaining` 里
