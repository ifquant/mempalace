# 背景

上一提交已经把 Rust `search` 的结构化 JSON 往 Python 的程序化接口靠齐了。  
接下来更自然的问题是：

- `status`
- `migrate`
- `repair`

这些命令虽然已经能返回 JSON，但它们的上下文字段还不够统一。  
对脚本和 agent 来说，最烦的不是“字段多一点”，而是不同命令各有各的组织方式。

# 主要目标

这次提交的目标是把 `status/migrate/repair` 的公共上下文再统一一层：

1. 增加稳定的 `kind`
2. 暴露明确的路径字段
3. 暴露版本字段
4. 让 CLI 和 MCP 都使用同样的结构化上下文

# 改动概览

主要改动如下：

- `rust/src/model.rs`
  - `Status` 新增：
    - `kind`
    - `sqlite_path`
    - `lance_path`
  - `MigrateSummary` 新增：
    - `kind`
    - `version`
  - `RepairSummary` 新增：
    - `kind`
    - `version`
- `rust/src/service.rs`
  - `status()` 现在会返回：
    - `kind = "status"`
    - `sqlite_path`
    - `lance_path`
  - `migrate()` 现在会返回：
    - `kind = "migrate"`
    - `version`
  - `repair()` 现在会返回：
    - `kind = "repair"`
    - `version`
- `rust/src/mcp.rs`
  - `mempalace_status` 现在也会回显：
    - `kind`
    - `sqlite_path`
    - `lance_path`
    - `version`
- `rust/tests/cli_integration.rs`
  - 补了 CLI 层对这些公共字段的断言
- `rust/tests/mcp_integration.rs`
  - 补了 MCP `status` 对这些字段的断言
- `rust/README.md`
  - 补充当前这些命令的公共上下文字段说明

# 关键知识

## 1. `kind` 字段对 agent 和脚本很值

很多时候人类看 JSON 一眼就知道这是什么命令的结果。  
但对程序来说，显式的 `kind` 会更稳：

- 可以直接 switch / dispatch
- 日志里更容易检索
- 多命令结果混在一起时更容易分流

所以它不是装饰字段，而是实际有用的协议字段。

## 2. 路径字段最好不要让调用方自己猜

如果 `status` 只告诉你 `palace_path`，调用方往往还得自己拼：

- `palace.sqlite3`
- `lance/`

这会带来两个问题：

- 每个调用方都要重复拼接规则
- 未来路径布局变更时更容易散掉

所以这次把 `sqlite_path` 和 `lance_path` 直接放进输出里，更利于后续演进。

# 补充知识

## 为什么 `version` 放在 `migrate/repair` 里也值得

因为这两个命令本来就偏运维和诊断。  
现场排查时，除了看：

- schema version
- embedding profile
- palace path

通常也会顺手看：

- 现在运行的是哪个 Rust 版本

把 `version` 放进去后，很多排查上下文就不需要额外再问一次。

## 为什么这里只统一“上下文字段”，而不是大改所有输出形状

因为当前更重要的是：

- 建一个稳定、可扩展的公共外壳

而不是一次把所有命令都完全推倒重排。  
先把：

- `kind`
- 路径
- 版本

这些横切字段统一，后面再继续细化各命令的专有字段，会更稳。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增关键验证：

- CLI：
  - `status` 包含 `kind/sqlite_path/lance_path`
  - `migrate` 包含 `kind/version`
  - `repair` 包含 `kind/version`
- MCP：
  - `mempalace_status` 包含 `kind/sqlite_path/lance_path/version`

# 未覆盖项

这次没有继续做：

- `status/migrate/repair` 的人类友好终端排版
- `mine` / `init` 输出形状统一
- 更细的命令结果 envelope 规范

所以这次提交的定位是：  
先把 Rust 运维类命令和 MCP `status` 的公共上下文结构统一起来，为后续继续收紧输出协议打底。
