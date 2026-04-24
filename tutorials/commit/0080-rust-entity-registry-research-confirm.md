## 背景

前一轮已经把 Rust 的 `entity_registry.json` 从只读 bootstrap 文件推进成了可写、可查的本地能力，但 Python 版还有一块紧邻能力没有迁过来：对未知词做 research，并在人工确认后写回 registry。

如果没有这层能力，Rust 版遇到陌生名字时只能停在 “unknown candidates”，还不能把外部研究结果沉淀进本地 registry。

## 主要目标

- 给 Rust `registry` CLI 增加 `research` 和 `confirm`
- 让 `entity_registry.json` 里的 `wiki_cache` 变成 Rust 的一等结构
- 对齐 Python `entity_registry.py` 的 research / confirm 主链路
- 保持测试稳定：单测和集成测试不能依赖真实 Wikipedia 网络可用性

## 改动概览

- 在 [rust/src/registry.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/registry.rs) 新增：
  - `RegistryResearchEntry`
  - `EntityRegistry::research()`
  - `EntityRegistry::confirm_research()`
  - Wikipedia summary 分类逻辑
- 在 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs) 新增：
  - `registry_research()`
  - `registry_confirm_research()`
- 在 [rust/src/model.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/model.rs) 新增：
  - `RegistryResearchResult`
  - `RegistryConfirmResult`
- 在 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs) 新增：
  - `registry research`
  - `registry confirm`
  - 对应 human 输出
- 在 [rust/tests/service_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/service_integration.rs) 和 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs) 补了 research/confirm 回归
- 在 [rust/Cargo.toml](/Users/dev/workspace2/agents_research/mempalace/rust/Cargo.toml) 增加了 `ureq`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. `wiki_cache` 最好先升格成强类型

之前 Rust 里 `wiki_cache` 只是 `serde_json::Value`，这在快速落地时很方便，但一旦开始真正提供 `research/confirm` 能力，就会出现两个问题：

- service / CLI 输出要不停手工拆字段
- 测试很难稳定表达“缓存里到底应该有什么”

这轮把它变成 `RegistryResearchEntry` 之后，registry 自己就能表达：

- `inferred_type`
- `confidence`
- `wiki_title`
- `wiki_summary`
- `confirmed`
- `confirmed_type`

这比继续在 service 层拼 JSON map 更稳，也更接近 Python 版把 wiki cache 作为 registry 正式组成部分的做法。

### 2. research 测试不能绑定外网

Python 版 research 真实会打 Wikipedia API，但 Rust 集成测试不能把成功与否绑在网络上，否则 CI 和本地验证都会变脆。

这轮做法是：

- 真实实现仍然支持打 Wikipedia
- 但 service / CLI 回归都先往 `entity_registry.json` 写入预置 `wiki_cache`
- 然后验证 `registry research` 能正确走缓存返回
- 再验证 `registry confirm` 会把缓存项提升进 people registry

也就是说：

- 功能面是真的
- 测试面是稳定可重复的

### 3. confirm 只把 “person” 写进 people registry

这轮刻意沿用了 Python 当前主路径：`confirm_research()` 真正落盘的是 `person`。

也就是：

- cache 里的研究结果可以是 `person/place/concept/ambiguous`
- 但正式推广到 registry 的主路径，目前先只把 `person` 写进 `people`

这样做的好处是先对齐 Python 当前行为，不在这个切片里顺手发散到更复杂的多实体写入模型。

## 补充知识

### 为什么用 `ureq`

这里选 `ureq` 而不是再拉一套更重的 HTTP client，原因很简单：

- registry research 是低频 CLI / service 能力
- 不需要 async client 池
- 这里要的是轻依赖、短路径、够用

所以这块维持“同步 research + 本地缓存”就够了。

### 为什么 `research --auto-confirm` 没直接写进 people

Python 的 `research()` 和 `confirm_research()` 是两步：

- `research` 负责研究并缓存
- `confirm_research` 负责真正确认并推广到 registry

Rust 这里保持同样分层，避免把“研究结果存在”和“用户确认写入”糊成一步。

## 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 还没有把 registry research / confirm 暴露到 MCP
- 还没有做 delete / rename / merge 这类 registry 运维操作
- 还没有做更完整的 place / concept 正式落盘模型
- 还没有做 interactive onboarding 里“发现 unknown candidate 后直接 research + confirm”的闭环
