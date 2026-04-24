# 背景

前几轮已经把 Rust 的这些命令输出逐步收紧了：

- `search`
- `status`
- `migrate`
- `repair`

但还有两个最常用的命令没有完全跟上同一套结构化风格：

- `init`
- `mine`

它们虽然已经能返回 JSON，但缺少统一的公共上下文字段。  
这样一来，脚本和 agent 在消费不同命令结果时，还是会遇到“每个命令都要单独适配”的问题。

# 主要目标

这次提交的目标很直接：

1. 给 `init` 增加统一的 `kind/version/schema_version`
2. 给 `mine` 增加统一的 `kind/project_path/palace_path/version`
3. 让 `mine` 也带上和 `search` 同风格的 `filters`
4. 用 CLI 和 service 测试把这些字段锁住

# 改动概览

主要改动如下：

- `rust/src/model.rs`
  - `InitSummary` 新增：
    - `kind`
    - `version`
    - `schema_version`
  - `MineSummary` 新增：
    - `kind`
    - `project_path`
    - `palace_path`
    - `version`
    - `filters`
- `rust/src/service.rs`
  - `init()` 现在返回：
    - `kind = "init"`
    - 当前 crate `version`
    - 当前 `schema_version`
  - `mine_project()` 现在返回：
    - `kind = "mine"`
    - `project_path`
    - `palace_path`
    - 当前 crate `version`
    - `filters`
- `rust/tests/service_integration.rs`
  - 对 `init` 和 `mine` 的新增字段补断言
- `rust/tests/cli_integration.rs`
  - 对 CLI 输出里的新增字段补断言
- `rust/README.md`
  - 记录 `init/mine` 也进入统一输出风格

# 关键知识

## 1. 最常用的命令最应该先统一 shape

`init` 和 `mine` 不只是“另外两个命令”，它们通常是：

- 第一次上手时最先跑的命令
- 自动化流程里最容易出现的命令

所以它们的输出如果不统一，实际摩擦会比一些低频命令更大。  
这也是为什么这次优先补它们，而不是继续扩更多次要能力。

## 2. `mine` 带 `filters` 是为了让 ingest 和 search 的调用习惯更一致

虽然 `mine` 不是搜索命令，但它同样受：

- `wing`

这类上下文影响。  
把这部分显式放进 `filters`，有两个好处：

- 与 `search` 的结构更一致
- 调用方更容易把“本次挖掘是在什么上下文下发生的”串起来

# 补充知识

## 为什么 `init` 没有加 `project_path`

因为在 Rust 当前架构里，service 层的 `init()` 关心的是：

- palace 目录
- SQLite / LanceDB 初始化

它并不天然知道“用户最初传入的 project dir”是什么。  
如果硬加这个字段，很容易引入假语义。

所以这次选择只给 `init` 增加确定无误的字段：

- `kind`
- `version`
- `schema_version`

## 为什么统一输出 shape 要一轮一轮做

因为每次都要同时考虑：

- service 层模型
- CLI 输出
- MCP 适配
- 测试
- 文档

如果一次改太多命令，风险会上升，也更难定位回归。  
拆成多轮提交，反而更稳。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增关键验证：

- service：
  - `init` 包含 `kind/version`
  - `mine` 包含 `kind/project_path/version`
- CLI：
  - `init` 输出包含 `kind/version/schema_version`
  - `mine` 输出包含 `kind/project_path/palace_path/filters`

# 未覆盖项

这次没有继续做：

- `doctor/prepare-embedding` 的 shape 统一
- `mine` 的人类可读终端排版
- 更正式的统一 output envelope 规范

所以这次提交的定位是：  
把最常用的 `init/mine` 也纳入 Rust 当前这条统一的结构化输出路线。
