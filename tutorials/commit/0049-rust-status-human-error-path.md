# 背景

`status --human` 已经能处理：

- no-palace
- 空 palace
- 正常非空 palace

但如果 palace 路径存在而 `palace.sqlite3` 已损坏，CLI 仍然会回退到原始 anyhow 错误。  
这和前面已经收紧过的 `repair --human`、`migrate --human` 风格不一致。

# 主要目标

- 让 `status --human` 在执行期错误时也输出 status 自己的人类可读错误文本
- 保持失败退出码

# 改动概览

- `Command::Status` 现在显式匹配 `app.status().await`
- human 模式下如果 `status()` 或 `taxonomy()` 失败：
  - 打印 `Status error: ...`
  - 打印下一步建议：检查 palace 文件，再重跑 `mempalace-rs status`
  - 保持 `exit code = 1`
- 新增 CLI 回归，覆盖损坏 SQLite 的 human status 错误路径

# 关键知识

- `status --human` 实际依赖两次读取：
  - `status()`
  - `taxonomy()`
  所以 human 错误分流要同时覆盖这两条路径。
- 当多条命令都属于“运维观察面”时，错误文案风格应该统一，用户才容易建立预期。

# 补充知识

- 很多 CLI 的一致性问题，不在成功路径，而在失败路径。
- 在命令层统一打印 `X error: ...`，比在各个 service 里塞展示逻辑更容易维护。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_status_human_reports_broken_sqlite_with_text_error
```

# 未覆盖项

- 这次没有改默认 JSON `status` 的执行期错误输出
- 这次没有改底层 `status` / `taxonomy` 逻辑
- 这次没有改 `doctor` / `prepare-embedding` 的执行期错误路径
