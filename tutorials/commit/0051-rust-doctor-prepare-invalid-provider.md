# 背景

`doctor --human` 和 `prepare-embedding --human` 已经能在命令执行成功后输出很完整的 embedding 诊断信息。  
但如果用户把 `MEMPALACE_RS_EMBED_PROVIDER` 设成不支持的值，CLI 仍然会直接回退到原始错误文本。

# 主要目标

- 让 `doctor --human` 在 invalid provider 场景下输出命令级可读错误文本
- 让 `prepare-embedding --human` 在 invalid provider 场景下输出命令级可读错误文本
- 保持失败退出码

# 改动概览

- `Command::Doctor` 现在对以下失败点做 human 分流：
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.doctor()`
- `Command::PrepareEmbedding` 现在对以下失败点做 human 分流：
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.prepare_embedding()`
- 新增两条 CLI 回归，覆盖 `MEMPALACE_RS_EMBED_PROVIDER=broken` 的 human 错误路径

# 关键知识

- 并不是所有错误都发生在真正的业务函数里。  
  `doctor` / `prepare-embedding` 这类命令，很可能先死在配置解析或 embedder 构建阶段。
- human 错误分流要放在命令层逐段包住，才能把不同阶段的失败都收口成一致体验。

# 补充知识

- 对依赖环境变量的命令，invalid provider 是一种很值得单独锁住的错误路径，因为它比网络失败更快、更稳定。
- 这里故意没有大范围重构所有命令的初始化路径，只收紧 `doctor` 和 `prepare-embedding` 两条线，能让切片更小更安全。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_doctor_human_reports_invalid_provider_with_text_error
cargo test --test cli_integration cli_prepare_embedding_human_reports_invalid_provider_with_text_error
```

# 未覆盖项

- 这次没有改默认 JSON 路径下的 invalid provider 错误输出
- 这次没有改 `init` / `status` / `repair` / `migrate` 对 invalid provider 的处理
- 这次没有改底层 embedding 逻辑
