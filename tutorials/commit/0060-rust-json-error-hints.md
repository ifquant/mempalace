# 背景

Rust CLI 之前已经把很多失败路径统一成了结构化 JSON 错误。  
但 payload 里大多只有 `error`，缺少和人类命令提示相对应的 `hint`。

# 主要目标

- 让默认 JSON 错误 payload 统一包含 `error + hint`
- 覆盖这些命令：
  - `init`
  - `search`
  - `mine`
  - `status`
  - `repair`
  - `migrate`
  - `doctor`
  - `prepare-embedding`

# 改动概览

- 给这些 JSON formatter 全部补上 `hint`：
  - `print_init_error_json()`
  - `print_search_error_json()`
  - `print_mine_error_json()`
  - `print_status_error_json()`
  - `print_repair_error_json()`
  - `print_migrate_error_json()`
  - `print_doctor_error_json()`
  - `print_prepare_embedding_error_json()`
- 收紧一组代表性 CLI 回归，显式断言：
  - `\"hint\":`
  - 命令对应的 rerun 提示

# 关键知识

- 结构化错误不该只服务机器。  
  `hint` 让 JSON surface 也能直接给人类或上层工具一个可操作的下一步。
- human surface 和 JSON surface 最好共享同一套建议语义，即使展示格式不同。

# 补充知识

- 当命令越来越多时，统一的 `error + hint` payload 比只返回一条错误字符串更容易被上层工具编排。
- 回归里断言 `\"hint\":` 本身就很有价值，因为它能防止后续重构把字段悄悄删掉。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_init_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_search_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_mine_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_status_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_repair_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_migrate_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_doctor_reports_invalid_provider_with_structured_error
cargo test --test cli_integration cli_prepare_embedding_reports_invalid_provider_with_structured_error
```

# 未覆盖项

- 这次没有改 human surface 文案
- 这次没有改底层 service 逻辑
- 这次没有改 Python 代码
