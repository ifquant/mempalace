# 背景

前几轮已经把 Rust 版这些命令的结构化输出逐步统一起来了：

- `search`
- `status`
- `migrate`
- `repair`
- `init`
- `mine`

但还有两条很常用的 runtime 诊断路径没有进入同一套外壳：

- `doctor`
- `prepare-embedding`

这会带来一个很现实的问题：  
当 agent 或脚本把这些命令结果并排处理时，只有它们还缺少统一的上下文字段。

# 主要目标

这次提交的目标是把：

- `doctor`
- `prepare-embedding`

也纳入当前 Rust 的统一输出路线：

1. 补 `kind`
2. 补 `sqlite_path`
3. 补 `lance_path`
4. 补 `version`

# 改动概览

主要改动如下：

- `rust/src/model.rs`
  - `DoctorSummary` 新增：
    - `kind`
    - `sqlite_path`
    - `lance_path`
    - `version`
  - `PrepareEmbeddingSummary` 新增：
    - `kind`
    - `sqlite_path`
    - `lance_path`
    - `version`
- `rust/src/embed.rs`
  - provider 侧创建 `DoctorSummary` 时先给出默认壳
  - 具体路径和版本由 service 层补齐
- `rust/src/service.rs`
  - `doctor()` 现在会把：
    - `sqlite_path`
    - `lance_path`
    - `version`
    回填进 summary
  - `prepare_embedding()` 成功和失败分支都会统一返回：
    - `kind = "prepare_embedding"`
    - `sqlite_path`
    - `lance_path`
    - `version`
- `rust/tests/service_integration.rs`
  - 增加 `doctor/prepare_embedding` 外壳字段断言
- `rust/tests/cli_integration.rs`
  - 增加 CLI 层对这些字段的断言
  - fastembed ignored smoke test 里也补了 `prepare-embedding` 的外壳断言
- `rust/README.md`
  - 记录 `doctor/prepare-embedding` 也已经进入统一输出风格

# 关键知识

## 1. 诊断类命令最需要稳定上下文

像 `doctor`、`prepare-embedding` 这种命令，调用它们的人通常不是在看“业务结果”，而是在看：

- 跑的是哪个 palace
- 对应哪个 SQLite / LanceDB
- 现在是哪个版本

所以给这类命令补统一外壳，信息价值比很多业务命令还高。

## 2. provider 层和 service 层的职责要分清

这次一个重要取舍是：

- provider 层知道 embedding runtime 细节
- service 层知道 palace 路径和应用版本

所以 `DoctorSummary` 的组装采取了分层策略：

- provider 先填 embedding 相关字段
- service 再补 palace 路径和版本字段

这样比把所有上下文都塞进 provider 更干净。

# 补充知识

## 为什么 `prepare-embedding` 的 `kind` 用下划线

当前 Rust 的结构化输出里，`kind` 更像程序字段，而不是用户文案。  
这里用：

- `prepare_embedding`

而不是 CLI 命令的连字符形式：

- `prepare-embedding`

是为了让它更适合被 JSON 消费和代码分支匹配。

## 为什么这次不把 `doctor` 也接进 MCP

因为这一轮目标只是收紧现有 CLI / service 输出契约。  
`doctor` 和 `prepare-embedding` 目前仍然主要是本地运维和 runtime 诊断入口。

先把它们在 CLI 侧做稳，再考虑是否要扩到 MCP，会更稳妥。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增关键验证：

- service：
  - `doctor` 包含 `kind/version/sqlite_path/lance_path`
  - `prepare_embedding` 包含 `kind/version/sqlite_path/lance_path`
- CLI：
  - `doctor` 输出包含这些字段
  - `prepare-embedding` 输出包含这些字段
  - fastembed ignored smoke test 也断言 `prepare-embedding` 的新外壳

# 未覆盖项

这次没有继续做：

- MCP 暴露 `doctor/prepare-embedding`
- `doctor` 的返回字段继续向 Python 高级能力扩展
- 更统一的全命令 output envelope 抽象

所以这次提交的定位是：  
把 runtime 诊断类命令也纳入 Rust 当前统一的结构化输出路线。
