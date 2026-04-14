# 背景

前一轮已经把 `doctor --human` 和 `prepare-embedding --human` 的 invalid-provider 错误路径收紧了。  
但默认非 human 模式下，这两个命令仍然会直接回退到原始 stderr，而不是稳定的结构化错误。

# 主要目标

- 让 `doctor` 在 invalid provider 场景下输出结构化 JSON 错误
- 让 `prepare-embedding` 在 invalid provider 场景下输出结构化 JSON 错误
- 保持 `--human` 路径不变

# 改动概览

- `Command::Doctor` 现在在这三类非 human 失败点上都转成 JSON：
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.doctor()`
- `Command::PrepareEmbedding` 现在在这三类非 human 失败点上都转成 JSON：
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.prepare_embedding()`
- 新增：
  - `print_doctor_error_json()`
  - `print_prepare_embedding_error_json()`
- 把一条基于错误前提的旧测试替换成真实有效的新回归：
  - `prepare-embedding` 并不会因为损坏 SQLite 必然失败
  - 更稳定的错误路径是 invalid provider

# 关键知识

- human surface 和 default JSON surface 都需要单独维护，不能因为一边好用就默认另一边也自动正确。
- 对“配置阶段就失败”的命令，structured JSON error 比裸 stderr 更利于脚本和测试消费。

# 补充知识

- 当测试失败暴露“命令其实不依赖 SQLite”这类事实时，应该修正测试假设，而不是硬把代码改成迎合错误测试。
- 为每个命令单独命名错误前缀，例如 `Doctor error:`、`Prepare embedding error:`，能让多命令工具的自动化日志更容易筛选。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_doctor_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_prepare_embedding_reports_invalid_provider_with_structured_error
```

# 未覆盖项

- 这次没有改 `search` / `mine` 的默认 JSON invalid-provider 路径
- 这次没有改底层 embedding 逻辑
- 这次没有改 `doctor` / `prepare-embedding` 的成功输出
