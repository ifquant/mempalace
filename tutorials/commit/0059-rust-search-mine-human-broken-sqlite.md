# 背景

`search` 和 `mine` 的 broken-sqlite 默认 JSON 错误已经有回归了。  
但同样的场景在 `--human` 文本 surface 上还没有单独锁住。

# 主要目标

- 锁住 `search --human` 在 broken-sqlite 场景下的文本错误
- 锁住 `mine --human` 在 broken-sqlite 场景下的文本错误

# 改动概览

- 新增：
  - `cli_search_human_reports_broken_sqlite_with_text_error`
  - `cli_mine_human_reports_broken_sqlite_with_text_error`
- 两条测试都断言：
  - 退出码是 `1`
  - stdout 里有对应命令前缀
  - `file is not a database` 存在
- `mine --human` 额外断言了下一步建议文本

# 关键知识

- 同一个错误场景需要分别覆盖 JSON 和 human 两条 surface，因为两边 formatter 是不同代码路径。
- `mine --human` 比 `search --human` 多一层“下一步建议”，所以断言也应该更完整。

# 补充知识

- 对 CLI 来说，“错误已经能打印出来”和“错误 surface 被测试锁住”是两件不同的事。
- broken-sqlite 这种夹具很适合复用，因为它稳定、快速、无需网络。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_search_human_reports_broken_sqlite_with_text_error
cargo test --test cli_integration cli_mine_human_reports_broken_sqlite_with_text_error
```

# 未覆盖项

- 这次没有改任何实现逻辑
- 这次没有改 JSON surface
- 这次没有改 Python 代码
