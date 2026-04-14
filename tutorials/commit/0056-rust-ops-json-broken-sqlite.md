# 背景

前一轮已经把 `init/status/repair/migrate` 在 invalid-provider 场景下的默认 JSON 错误路径统一了。  
但这组命令在“palace 路径存在且 `palace.sqlite3` 已损坏”的执行期错误上，还缺少默认 JSON 回归覆盖。

# 主要目标

- 锁住 `init` 在 broken-sqlite 场景下的结构化 JSON 错误
- 锁住 `status` 在 broken-sqlite 场景下的结构化 JSON 错误
- 锁住 `repair` 在 broken-sqlite 场景下的结构化 JSON 错误
- 锁住 `migrate` 在 broken-sqlite 场景下的结构化 JSON 错误

# 改动概览

- 新增 4 条 CLI 回归，分别覆盖：
  - `cli_init_reports_broken_sqlite_with_structured_error`
  - `cli_status_reports_broken_sqlite_with_structured_error`
  - `cli_repair_reports_broken_sqlite_with_structured_error`
  - `cli_migrate_reports_broken_sqlite_with_structured_error`
- 这些测试都断言：
  - 退出码是 `1`
  - stdout 里有 `{"error": ...}`
  - 错误前缀和 `file is not a database` 都存在

# 关键知识

- 有些切片不是“新实现逻辑”，而是把已经存在的错误 surface 用真实场景锁住。  
  这种工作对稳定后续重构很重要。
- broken-sqlite 和 invalid-provider 是两类完全不同的失败：
  - 一个是本地状态损坏
  - 一个是配置无效
  两类都需要单独覆盖。

# 补充知识

- 对本地优先工具来说，损坏的本地状态比网络失败更常见，所以值得单独做稳定回归。
- `file is not a database` 这种 SQLite 经典错误，特别适合作为“坏 palace”夹具的固定断言。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_init_reports_broken_sqlite_with_structured_error
cargo test --test cli_integration cli_status_reports_broken_sqlite_with_structured_error
cargo test --test cli_integration cli_repair_reports_broken_sqlite_with_structured_error
cargo test --test cli_integration cli_migrate_reports_broken_sqlite_with_structured_error
```

# 未覆盖项

- 这次没有改任何 Python 代码
- 这次没有改底层 service 逻辑
- 这次没有新增新的错误 formatter，只是把既有 JSON error 路径用真实场景锁住
