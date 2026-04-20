# 背景

上一轮已经把 parity ledger 和 README 的“remaining work”表述收窄到更真实的范围：主要剩下的是 README/example 的 truth-in-docs，而不是大面积 help 契约还没锁住。

接下来最自然的一步就是把 `rust/README.md` 里的示例命令补齐到更接近当前真实 CLI 面。否则 README 会继续显得像只覆盖了部分命令，而不是已经补过多轮 help/test 的当前状态。

# 主要目标

- 扩充 `rust/README.md` 里的验证命令示例
- 让 README 示例更贴近当前已经被 help/test 锁住的 CLI 面

# 改动概览

- 更新 `rust/README.md`
  - 在 “Useful verification command” 中新增：
    - `status --human`
    - `wake-up --human`
    - `search ... --human`
    - `split ... --dry-run`
    - `mcp --setup`

# 关键知识

README 示例不是随便举几个命令就够了。  
它承担的是“让后来的人快速感知当前表面到底多大”的职责。

当 help/test 已经覆盖了：

- read 面
- maintenance 面
- project-facing transcript/bootstrap 面
- MCP setup/serve 面

README 示例就应该相应反映这些入口，而不是停留在一个更早、更窄的命令集合上。

# 补充知识

这类文档修正和功能开发不同，不一定要增加新的代码测试。  
更关键的是做交叉检查：

1. README 示例是否对应当前真实命令
2. 示例是否和 CLI help / tests 的覆盖重点一致
3. ledger 里的“README/example consistency audit”是否真的在被推进

# 验证

- 交叉检查 `rust/README.md` 中新增示例与当前 CLI surface 是否一致
- 交叉检查这些示例是否都已有对应 help/test 覆盖或稳定命令表面
- 本次是 README example truth pass，没有新增 Rust 运行时代码改动

# 未覆盖项

- 这次没有修改 `rust/src/`
- 这次没有修改 `rust/tests/`
- 这次没有继续改 `docs/parity-ledger.md`
- 更深的非 CLI 行为 parity 审计仍然保留在 ledger 的 `remaining` 里
