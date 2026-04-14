# 背景

前面已经把 `search/mine/doctor/prepare-embedding` 的默认 JSON invalid-provider 错误路径逐步收紧了。  
剩下的 `init`、`status`、`repair`、`migrate` 仍然混用原始 stderr 和结构化错误。

# 主要目标

- 让 `init` 在 invalid provider 场景下输出结构化 JSON 错误
- 让 `status` 在 invalid provider 场景下输出结构化 JSON 错误
- 让 `repair` 在 invalid provider 场景下输出结构化 JSON 错误
- 让 `migrate` 在 invalid provider 场景下输出结构化 JSON 错误

# 改动概览

- 新增：
  - `print_init_error_json()`
  - `print_status_error_json()`
  - `print_repair_error_json()`
  - `print_migrate_error_json()`
- `Command::Init` 现在对以下 non-human 失败点输出 JSON：
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.init()`
- `Command::Status` 现在对以下 non-human 失败点输出 JSON：
  - `AppConfig::resolve()`
  - `app.status()`
- `Command::Repair` 现在对以下 non-human 失败点输出 JSON：
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.repair()`
- `Command::Migrate` 现在对以下 non-human 失败点输出 JSON：
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.migrate()`
- 新增 4 条 CLI 回归，覆盖这 4 个命令在 `MEMPALACE_RS_EMBED_PROVIDER=broken` 下的结构化错误输出

# 关键知识

- 当一组命令都依赖同一套配置层时，错误 surface 也应该统一，不然用户会觉得工具行为随机。
- `status` 比较特殊：它还有 no-palace 这个正常分支，所以 invalid-provider 错误只该发生在更早的 config 层，而不是覆盖 no-palace 语义。

# 补充知识

- 统一的 `{"error":"<Command> error: ..."}` 形状，既方便测试，也方便以后让外部脚本稳定解析。
- 运维命令的错误体验一旦不统一，排查时心智成本会明显上升，因为用户需要记每个子命令的失败风格。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_init_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_status_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_repair_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_migrate_reports_invalid_provider_with_structured_error
```

# 未覆盖项

- 这次没有改任何 Python 代码
- 这次没有改底层 service 逻辑
- 这次没有收紧非 invalid-provider 的默认 JSON 错误路径
