# 背景

运维命令这组实现已经具备 human invalid-provider 错误分流，但之前只有默认 JSON 路径被显式锁住。  
如果后面有人改动命令入口，human surface 有回退风险。

# 主要目标

- 给 `init --human` 增加 invalid-provider 回归
- 给 `status --human` 增加 invalid-provider 回归
- 给 `repair --human` 增加 invalid-provider 回归
- 给 `migrate --human` 增加 invalid-provider 回归

# 改动概览

- 新增 4 条 CLI 回归，统一覆盖 `MEMPALACE_RS_EMBED_PROVIDER=broken`
- 每条测试都断言：
  - 退出码是 `1`
  - stdout 里有对应命令前缀
  - stdout 里有 `Unsupported embedding provider: broken`
  - stdout 里有该命令自己的下一步建议

# 关键知识

- 对 CLI 来说，“实现存在”不等于“行为安全”。  
  如果没有测试锁住，后续重构入口逻辑时很容易把 human 分流不小心删掉。
- 同一类错误最好在同一批命令上成组覆盖，能更快看出哪一个子命令行为走偏了。

# 补充知识

- invalid-provider 这类错误非常适合做快速 CLI 回归，因为不依赖网络、数据库内容或模型缓存。
- 用环境变量触发配置错误，比改 fixture 文件更轻，测试也更快。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_init_human_reports_invalid_provider_with_text_error
cargo test --test cli_integration cli_status_human_reports_invalid_provider_with_text_error
cargo test --test cli_integration cli_repair_human_reports_invalid_provider_with_text_error
cargo test --test cli_integration cli_migrate_human_reports_invalid_provider_with_text_error
```

# 未覆盖项

- 这次没有改任何实现逻辑
- 这次没有改默认 JSON surface
- 这次没有改 Python 代码
