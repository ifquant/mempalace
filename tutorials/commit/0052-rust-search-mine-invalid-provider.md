# 背景

前面已经把 `doctor --human` 和 `prepare-embedding --human` 的 invalid-provider 错误路径收紧了。  
但 `search --human` 和 `mine --human` 还会在 provider 配置坏掉时直接回退到原始错误。

# 主要目标

- 让 `search --human` 在 invalid provider 场景下输出命令级可读错误文本
- 让 `mine --human` 在 invalid provider 场景下输出命令级可读错误文本
- 保持失败退出码

# 改动概览

- `Command::Search` 现在对以下失败点做 human 分流：
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.search()`
- `Command::Mine` 现在对以下失败点做 human 分流：
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.mine_project()` / `app.mine_project_with_progress()`
- 新增两条 CLI 回归，覆盖 `MEMPALACE_RS_EMBED_PROVIDER=broken` 的 human 错误路径

# 关键知识

- 配置错误常常发生在真正业务逻辑之前，所以 human 错误分流必须包住“命令入口到业务调用”整条链。
- `mine` 有两条执行路径：
  - 普通 `mine_project()`
  - 带进度的 `mine_project_with_progress()`
  所以错误收口时要先把两条路径统一成同一个 `Result`。

# 补充知识

- 对最常用的命令，invalid provider 这种“启动即失败”的路径比深层 query failure 更值得优先收紧。
- `search --human` 已经有 query-time 错误文本，这次只是把更早的 config/app 初始化失败也并进同一套体验。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_search_human_reports_invalid_provider_with_text_error
cargo test --test cli_integration cli_mine_human_reports_invalid_provider_with_text_error
```

# 未覆盖项

- 这次没有改默认 JSON 路径下的 invalid provider 错误输出
- 这次没有改 `init/status/repair/migrate` 对 invalid provider 的处理
- 这次没有改底层搜索或挖掘逻辑
