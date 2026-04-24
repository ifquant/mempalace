# 背景

`repair --human` 已经能处理两种情况：

- palace 不存在：直接给出提示
- palace 健康或可诊断：输出 readable summary

但如果 palace 路径存在，而 SQLite 文件本身已经损坏，CLI 仍然会把原始 anyhow 错误打到 `stderr`，没有命令级的人类诊断。

# 主要目标

- 让 `repair --human` 在执行期报错时也输出 repair 自己的可读错误文本
- 保持失败退出码，不把真实错误吞掉

# 改动概览

- `Command::Repair` 现在像 `search` 一样显式匹配 `app.repair().await`
- human 模式下如果 `repair()` 出错：
  - 打印 `Repair error: ...`
  - 打印下一步建议：检查 palace 文件，再重跑 `mempalace-rs repair`
  - 保持 `exit code = 1`
- 新增 CLI 回归，覆盖“损坏 SQLite 文件”的 human 错误路径

# 关键知识

- CLI command 的“业务错误展示”最好在命令层统一处理，而不是把底层 `anyhow` 默认展示直接暴露给用户。
- “命令失败”不等于“只能输出 stderr”。  
  很多 CLI 会先打印结构化或可读诊断，再带非零退出码返回。

# 补充知识

- `repair` 这种运维命令尤其需要命令级错误文本，因为用户最常在半损坏状态下运行它。
- 用一个包含伪造 `palace.sqlite3` 文本文件的目录，就能稳定覆盖“文件存在但不是 SQLite 数据库”的失败路径。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_repair_human_reports_issue_summary_and_next_step
```

# 未覆盖项

- 这次没有改默认 JSON `repair` 的执行期错误输出
- 这次没有改 `repair` 的底层诊断逻辑
- 这次没有改 `migrate` / `status` 的执行期错误路径
