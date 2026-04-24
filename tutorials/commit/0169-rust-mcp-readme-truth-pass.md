# 背景

README 前面几轮已经逐步把 parity 状态、remaining work、命令示例、阶段口吻都收得更接近当前事实，但 MCP 这一节的标题还保留着一个偏旧的 framing：`Current MCP compatibility notes`。

现在的 Rust MCP 状态已经不只是“兼容 Python MCP”，还明显包含：

- Python MCP 公共工具面的完整覆盖
- 多组 Rust-only MCP 扩展面
- 一些本地优先的行为差异

所以如果章节标题继续只写 `compatibility`，就会低估这节实际承载的信息范围。

# 主要目标

- 把 README 里的 MCP 小节标题和开头改成更符合当前状态的 truth-in-docs 表述

# 改动概览

- 更新 `rust/README.md`
  - 将 `Current MCP compatibility notes` 改为 `Current MCP parity and extension notes`
  - 在该节开头补两条 framing：
    - 当前 Python MCP 公共工具面是 Rust MCP 的子集
    - 下方内容聚焦 parity、Rust-only 扩展面、以及本地优先差异

# 关键知识

文档标题不只是装饰，它决定读者如何理解下面那一串 bullet 的性质。  
当一个章节既在讲“已经对齐的东西”，也在讲“Rust 独有的扩展面”和“有意差异”，继续把标题写成单纯的 `compatibility`，会让内容和 framing 失配。

# 补充知识

Parity 文档常见的一个漂移模式是：

1. 一开始只是在讲兼容
2. 后来加入了扩展和差异
3. 章节标题却一直没改

结果就是正文越来越真实，标题却越来越误导。  
这次修的就是这种“标题层 truth drift”。

# 验证

- 交叉检查 `rust/README.md` 的 MCP 小节标题与 [docs/parity-ledger.md](/Users/dev/workspace2/agents_research/mempalace/docs/parity-ledger.md) 当前结论是否一致
- 确认新增 framing 没有引入新的功能承诺，只是在更准确地描述当前 Rust MCP 状态
- 本次是文档 truth pass，没有新增 Rust 运行时代码改动

# 未覆盖项

- 这次没有修改 `rust/src/`
- 这次没有修改 `rust/tests/`
- 这次没有修改 `docs/parity-ledger.md`
- 更深的非 CLI / non-MCP 行为 parity 审计仍然保留在 ledger 的 `remaining` 里
