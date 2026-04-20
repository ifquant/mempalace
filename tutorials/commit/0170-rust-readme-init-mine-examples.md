# 背景

README 的验证命令示例前几轮已经补进了 read、maintenance、registry、helper、MCP 这些入口，但一个明显的小缺口还在：最基础的 `init` 和 `mine` 反而没有直接列在这组“Useful verification command”里。

这会让 README 的示例集合看起来像已经覆盖了很多外围和高级命令，却没有把最基本的 project bootstrap / ingest 主线显式摆出来。

# 主要目标

- 把 `init` 示例补进 README
- 把 `mine` 示例补进 README

# 改动概览

- 更新 `rust/README.md`
  - 在 “Useful verification command” 顶部新增：
    - `cargo run -- --palace /tmp/mempalace init /path/to/project --human`
    - `cargo run -- --palace /tmp/mempalace mine /path/to/project --progress`

# 关键知识

示例集合的“代表性”很重要。  
即使文档已经覆盖了很多高级或分支命令，如果最基础的主线命令没有明确出现，读者还是会低估这份示例列表的实用性。

对 MemPalace Rust 来说，`init -> mine` 仍然是最基本的起步链路，因此应该直接出现在验证命令列表里，而不是只隐含在其他段落或 first-run flow 里。

# 补充知识

README 示例和“推荐流程”不是一回事：

- 推荐流程强调顺序和环境准备
- 验证命令示例强调“有哪些真实入口值得直接试”

所以即使下面已经有 fastembed first-run flow，顶上的示例列表仍然应该显式包含 `init` / `mine`。

# 验证

- 交叉检查 `rust/README.md` 中新增示例是否都对应当前真实 CLI surface
- 确认 `init` 与 `mine` 已有稳定命令表面与 help/test 覆盖
- 本次是 README example truth pass，没有新增 Rust 运行时代码改动

# 未覆盖项

- 这次没有修改 `rust/src/`
- 这次没有修改 `rust/tests/`
- 这次没有修改 `docs/parity-ledger.md`
- 更深的非 CLI 行为 parity 审计仍然保留在 ledger 的 `remaining` 里
