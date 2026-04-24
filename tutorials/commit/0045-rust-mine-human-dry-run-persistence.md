# 背景

Rust 版 `mine --human --dry-run` 已经会显示 `Mode: DRY RUN`，但用户仍然需要自己推断“这次没有真正写入 palace”。  
对新用户来说，这个信息最好直接写出来。

# 主要目标

- 让 `mine --human --dry-run` 明确说明这是 preview-only
- 避免用户把 dry-run 误解为已经真的落盘

# 改动概览

- 在 `print_mine_human()` 的 dry-run 分支里新增：
  - `Drawers previewed: ...`
  - `Persistence:     preview only, no drawers were written`
- 新增 CLI 回归，锁住 dry-run human 输出

# 关键知识

- 有些状态虽然能从上下文“推理出来”，但 CLI 最好直接说出来。  
  `Mode: DRY RUN` 说明了模式，`Persistence: preview only` 才真正说明了结果；`Drawers previewed` 则避免把预估数量误读成已落盘数量。
- 展示层增强不一定需要改结构体。  
  如果现有字段已经足够表达语义，直接在 formatter 里补一层解释通常更稳。

# 补充知识

- 终端工具里，关于“是否真的写盘”这类信息属于高风险歧义点，应该优先消除。
- 测试 dry-run human 文案时，最好同时断言 `Drawers previewed` 和新的 `Persistence:`，并确认不再出现 `Drawers filed`。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_mine_human_dry_run_reports_preview_only
```

# 未覆盖项

- 这次没有改 dry-run 的 JSON 输出
- 这次没有改实际 mine/dry-run 逻辑
- 这次没有实现更多 Python `convos` 能力
