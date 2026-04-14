# 背景

`init --human` 已经能输出人类可读的初始化摘要，但如果 palace 路径下已经存在损坏的 `palace.sqlite3`，CLI 仍然会直接掉回原始 anyhow 错误。  
这和前面已经收紧过的 `status/repair/migrate --human` 错误路径不一致。

# 主要目标

- 让 `init --human` 在执行期错误时也输出 init 自己的人类可读错误文本
- 保持失败退出码

# 改动概览

- `Command::Init` 现在显式匹配 `app.init().await`
- human 模式下如果初始化执行失败：
  - 打印 `Init error: ...`
  - 打印下一步建议：检查 palace 路径和 SQLite 文件，再重跑 `mempalace-rs init <dir>`
  - 保持 `exit code = 1`
- 新增 CLI 回归，覆盖“已有损坏 SQLite 文件”的 human init 错误路径

# 关键知识

- `init` 虽然是“创建”命令，但并不意味着它一定只会遇到空目录；已有半损坏 palace 也是常见状态。
- 命令层错误分流最适合处理这类 human output，不需要把展示逻辑塞进 `service::App::init()`。

# 补充知识

- 对很多本地工具来说，最常见的坏状态不是“什么都没有”，而是“目录已经有一半旧文件”。
- `init` 的错误建议里同时提 palace path 和 SQLite file，比只说其中一个更实用。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_init_human_reports_broken_sqlite_with_text_error
```

# 未覆盖项

- 这次没有改默认 JSON `init` 的执行期错误输出
- 这次没有改底层初始化逻辑
- 这次没有改 `doctor` / `prepare-embedding` 的执行期错误路径
