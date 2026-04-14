# 背景

Rust 版 `status --human` 已经能显示非空 palace 的 wing/room 分布，但在“palace 已初始化、还没有任何 drawer”时，终端上缺少一句明确结论。  
用户只能从 `0 drawers` 自己猜测当前状态。

# 主要目标

- 让 `status --human` 在空 palace 场景下直接说清楚“已初始化但还是空的”
- 顺手给出下一步动作：`mempalace mine <dir>`

# 改动概览

- 在 `print_status_human()` 里增加 `summary.total_drawers == 0` 分支
- 空 palace 时会输出：
  - `Palace is initialized but still empty.`
  - `Run: mempalace mine <dir>`
- 新增 CLI 回归，锁住这个 human 输出

# 关键知识

- “不存在 palace”和“存在但为空”是两个不同状态，CLI 最好显式区分。  
  前者更像初始化问题，后者更像尚未 ingest 数据。
- 这类提示最好直接放在标题下面，用户扫一眼就能看到，不用从空 taxonomy 推断。

# 补充知识

- 对人类终端用户来说，下一步动作提示通常比更多统计字段更有价值。
- 这种切片完全可以只改 formatter 和 CLI test，不必触碰底层 `Status` 结构。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_status_human_empty_palace_reports_next_step
```

# 未覆盖项

- 这次没有改 `status` 的 JSON 输出
- 这次没有改 non-human CLI
- 这次没有改 Python CLI
