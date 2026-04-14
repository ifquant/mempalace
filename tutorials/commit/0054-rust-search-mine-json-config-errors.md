# 背景

前一轮已经把 `search --human` 和 `mine --human` 的 invalid-provider 错误路径收紧成命令级可读文本。  
但默认非 human 模式下，这两个主命令仍然会在部分配置失败点上直接回退到原始 stderr。

# 主要目标

- 让 `search` 在 invalid provider 场景下输出结构化 JSON 错误
- 让 `mine` 在 invalid provider 场景下输出结构化 JSON 错误
- 保持 `--human` 路径不变

# 改动概览

- `Command::Search` 现在在 `AppConfig::resolve()` 的 non-human 失败分支里输出：
  - `{"error":"Search error: ..."}`
- `Command::Mine` 现在在以下 non-human 失败点都输出：
  - `{"error":"Mine error: ..."}`
  - `AppConfig::resolve()`
  - `App::new()`
  - `app.mine_project()` / `app.mine_project_with_progress()`
- 新增：
  - `print_mine_error_json()`
- 新增两条 CLI 回归，覆盖 `search` / `mine` 在 invalid provider 场景下的结构化错误输出

# 关键知识

- human surface 和 default JSON surface 要分别锁住，不能只测一边。
- `mine` 的执行路径更多，所以如果不把错误先统一成一个 `Result`，很容易漏掉某一支分支的 JSON 错误输出。

# 补充知识

- 对主命令来说，structured JSON error 不是“锦上添花”，而是脚本可集成性的基础。
- 错误前缀命名要稳定：`Search error:`、`Mine error:`，这样测试和日志过滤都简单。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_search_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_mine_reports_invalid_provider_with_structured_error
```

# 未覆盖项

- 这次没有改 `init/status/repair/migrate` 的默认 JSON invalid-provider 路径
- 这次没有改底层搜索或挖掘逻辑
- 这次没有改任何 Python 代码
