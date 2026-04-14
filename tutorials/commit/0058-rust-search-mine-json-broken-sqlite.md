# 背景

前面已经把 `search` 和 `mine` 的 invalid-provider 默认 JSON 错误路径锁住了。  
但这两个主命令在“palace 路径存在且 `palace.sqlite3` 已损坏”时，还缺少专门回归。

# 主要目标

- 锁住 `search` 在 broken-sqlite 场景下的结构化 JSON 错误
- 锁住 `mine` 在 broken-sqlite 场景下的结构化 JSON 错误

# 改动概览

- 新增：
  - `cli_search_reports_broken_sqlite_with_structured_error`
  - `cli_mine_reports_broken_sqlite_with_structured_error`
- 两条测试都断言：
  - 退出码是 `1`
  - stdout 里有 `{"error": ...}`
  - 对应命令前缀存在
  - `file is not a database` 存在

# 关键知识

- `search` 和 `mine` 虽然是主功能命令，但它们的失败模式和运维命令一样，也会受本地 SQLite 状态损坏影响。
- 把 broken-sqlite 和 invalid-provider 分开测，能更精确地区分“状态损坏”和“配置错误”两类回归。

# 补充知识

- 对本地优先项目来说，SQLite 损坏是一条应该长期保留的高价值回归路径。
- 只补测试、不改实现，也是很有效的“收紧”动作，因为它能把既有行为固化成仓库事实。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_search_reports_broken_sqlite_with_structured_error
cargo test --test cli_integration cli_mine_reports_broken_sqlite_with_structured_error
```

# 未覆盖项

- 这次没有改任何实现逻辑
- 这次没有改 human surface
- 这次没有改 Python 代码
