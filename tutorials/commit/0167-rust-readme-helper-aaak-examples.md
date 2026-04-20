# 背景

上一轮已经把 README 的验证命令示例扩到更多 read/project/MCP 入口，但示例集合里仍然漏了几类已经稳定存在的表面：

- AAAK 压缩
- helper instructions
- harness hook run

如果 README 继续缺这几类入口，后来的人还是会从示例上误判 Rust CLI 的真实宽度。

# 主要目标

- 把 AAAK 压缩示例补进 README
- 把 helper instructions 示例补进 README
- 把 hook run 示例补进 README

# 改动概览

- 更新 `rust/README.md`
  - 在 “Useful verification command” 中新增：
    - `compress --dry-run`
    - `instructions help`
    - `hook run --hook session-start --harness codex`

# 关键知识

README 示例的覆盖面不应该只盯“主命令”。  
像 `compress`、`instructions`、`hook run` 这种命令虽然不一定是第一次初始化时就会用到，但它们依然属于当前真实可见表面。

如果示例集合长期只覆盖部分命令，README 会逐渐变成一个“窄视角入口页”，而不是当前 CLI 面的可信样本。

# 补充知识

`hook run` 这类命令和普通 CLI 不太一样，因为它需要 stdin 输入。  
README 里如果要给出这类示例，最好直接放一个最小可运行片段，例如：

- `printf '{"session_id":"demo"}' | ...`

这样读者看到的不只是命令名，而是完整调用方式。

# 验证

- 交叉检查 `rust/README.md` 中新增示例是否都对应当前真实 CLI surface
- 交叉检查 `compress`、`instructions`、`hook run` 是否都已有稳定命令表面和 help/test 覆盖
- 本次是 README example truth pass，没有新增 Rust 运行时代码改动

# 未覆盖项

- 这次没有修改 `rust/src/`
- 这次没有修改 `rust/tests/`
- 这次没有修改 `docs/parity-ledger.md`
- 更深的非 CLI 行为 parity 审计仍然保留在 ledger 的 `remaining` 里
