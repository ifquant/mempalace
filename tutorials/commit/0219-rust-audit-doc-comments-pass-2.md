# 背景

这次是 Rust 审计注释整理的第二个切片，范围集中在 `registry`、`knowledge_graph`、`bootstrap`、`onboarding` 和它们对应的 CLI facade。前一轮已经把 storage / maintenance 的边界补清楚了，但 reviewer 仍然很难快速回答这几个问题：

- `entity_registry.json` 的读写、lookup、research 各自在哪一层
- KG 只是一个薄 facade，还是自己还藏了额外语义
- `init` 和 `onboarding` 都会写哪些 bootstrap 文件，遇到已有文件时谁负责“保留而不是覆盖”
- onboarding 的自动检测结果和显式输入怎么 merge，alias / person / project 的去重规则在哪里

这轮的任务仍然有硬约束：

- 只改 Task 4 指定 write set
- 只做注释整理，不引入行为改动
- 不碰 `python/uv.lock`
- 不碰 `docs/superpowers/` 下的计划文件

# 主要目标

把 registry / KG / bootstrap / onboarding 这条“本地世界模型”链路补到 reviewer 可以顺着文件名快速定位边界：

- `registry.rs` 只是 split facade，具体读写逻辑分布在哪些子文件
- `RegistryRuntime` 暴露了哪些真正对外的 read/write/research 入口
- `KnowledgeGraph` 和 SQLite KG 层的职责边界是什么
- `bootstrap_project` 与 `run_onboarding` 在生成文件、保留已有文件、合并检测结果时分别承担什么责任
- CLI 层只是路由和打印，不额外承载业务语义

# 改动概览

- 给 major subsystem / facade 文件补了模块级 `//!` 注释
  - `rust/src/registry.rs`
  - `rust/src/registry_io.rs`
  - `rust/src/registry_lookup.rs`
  - `rust/src/registry_mutation.rs`
  - `rust/src/registry_research.rs`
  - `rust/src/registry_runtime.rs`
  - `rust/src/registry_types.rs`
  - `rust/src/knowledge_graph.rs`
  - `rust/src/bootstrap.rs`
  - `rust/src/bootstrap_docs.rs`
  - `rust/src/bootstrap_files.rs`
  - `rust/src/onboarding.rs`
  - `rust/src/onboarding_prompt.rs`
  - `rust/src/onboarding_support.rs`
  - `rust/src/project_cli_bootstrap.rs`
  - `rust/src/project_cli_bootstrap_init.rs`
  - `rust/src/project_cli_bootstrap_onboarding.rs`
  - `rust/src/project_cli_bootstrap_support.rs`
  - `rust/src/registry_cli.rs`
  - `rust/src/registry_cli_read.rs`
  - `rust/src/registry_cli_write.rs`
  - `rust/src/registry_cli_research.rs`
  - `rust/src/registry_cli_support.rs`
- 给关键 public anchor 补了 `///`
  - `RegistryRuntime`
  - `KnowledgeGraph`
  - `InitBootstrap`
  - `OnboardingRequest`
  - `RegistryCommand`
  - registry / bootstrap / onboarding / CLI 的主要公开函数
- 只在几处容易误读的逻辑旁边补了稀疏 inline comment
  - registry lookup 时本地 registry 优先于 wiki cache
  - KG facade 调用 SQLite 时，normalized entity ID 在底层生成
  - `bootstrap_project` 遇到已有文件时保留本地版本，不主动覆盖
  - onboarding auto-detected people/projects merge 时只补未显式提供的条目

# 关键知识

这一轮最值得记住的是：`registry / bootstrap / onboarding` 不是一条完全对称的链路，而是三个不同职责层。

- `registry`
  - 负责“项目里已经承认了哪些实体”
  - 包括 `people / projects / ambiguous_flags / wiki_cache`
  - lookup、learn、manual add、research confirm 都落在这里
- `bootstrap`
  - 负责“第一次初始化时生成哪些基础文件”
  - 重点是缺失时写入、已有时保留
  - 不承担交互式 merge 策略
- `onboarding`
  - 负责“带用户意图地构造世界模型”
  - 会做 prompt、default、dedupe、auto-detect merge
  - 最后也会写 registry / docs / config，但它的价值在于输入整理，不只是写文件

所以审计时不要把它们混成一个层：

- 想看 JSON registry 的 durable shape，看 `registry_types.rs`
- 想看 registry 真正怎么读/写/查，看 `registry_io.rs` / `registry_lookup.rs` / `registry_mutation.rs`
- 想看 `init` 为什么不覆盖已有文件，看 `bootstrap.rs`
- 想看 onboarding 自动检测和显式参数如何合并，看 `onboarding_support.rs` + `onboarding.rs`

# 补充知识

1. Rust 的 facade 文件如果只有 `pub use` 或少量路由代码，仍然值得补 `//!`。原因不是“增加字数”，而是让 reviewer 一眼知道“这里是总入口，细节在旁边的 split 模块”，否则这种薄文件最容易被误判成无关代码。

2. 注释“保留已有文件”时，最好把语义写成 additive / preserve existing，而不是简单写“if exists skip”。前者能帮助 reviewer 理解这是产品决策：`init` 不应该踩掉用户已经编辑过的 bootstrap 文档。

# 验证

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test --test parity_registry_kg_ops --quiet
cargo test --test service_integration registry_summary_lookup_and_learn_work --quiet
```

# 未覆盖项

- 没有修改 `python/` 实现，也没有触碰 `python/uv.lock`
- 没有修改 `docs/superpowers/` 下的任何计划或执行文档
- 没有修改 Task 4 write set 之外的 Rust 模块，例如 `service.rs`、`storage/sqlite_kg.rs`、`mcp_*` 或 mining/normalize 相关文件
