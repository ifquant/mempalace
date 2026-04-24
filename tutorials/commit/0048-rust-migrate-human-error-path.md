# 背景

`migrate --human` 已经能处理：

- palace 不存在
- 正常迁移

但如果 palace 路径存在，而 `palace.sqlite3` 已经损坏，CLI 仍然会回退到原始 anyhow 错误输出。  
这对运维体验不够好。

# 主要目标

- 让 `migrate --human` 在执行期出错时也输出 migrate 自己的人类可读错误文本
- 保持失败退出码，不把错误伪装成成功

# 改动概览

- `Command::Migrate` 现在显式匹配 `app.migrate().await`
- human 模式下如果迁移执行失败：
  - 打印 `Migrate error: ...`
  - 打印下一步建议：检查 SQLite 文件，再重跑 `mempalace-rs migrate`
  - 保持 `exit code = 1`
- 新增 CLI 回归，覆盖损坏 SQLite 的 human migrate 错误路径

# 关键知识

- 对运维命令来说，no-palace、summary、execution-error 往往是三条不同的 surface，不能只处理前两条。
- 在 CLI 层做错误分流，比在底层 service 里塞展示逻辑更干净。

# 补充知识

- `migrate` 和 `repair` 的错误体验应该保持一致，这样用户在修 palace 时不会面对两套截然不同的失败风格。
- 用文本文件伪装成 `palace.sqlite3` 是覆盖 SQLite 损坏错误路径的最低成本夹具。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_migrate_human_reports_broken_sqlite_with_text_error
```

# 未覆盖项

- 这次没有改默认 JSON `migrate` 的执行期错误输出
- 这次没有改底层迁移逻辑
- 这次没有改 `status` / `doctor` 的执行期错误路径
