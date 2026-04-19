# 背景

前几轮已经把 registry 拆成：

- `registry_types`
- `registry_research`
- `registry`

但 `rust/src/registry.rs` 里仍然同时放着两类不同节奏的逻辑：

- 持久化、seed、bootstrap、learn、manual mutation
- lookup、歧义消解、query-side extraction

这会让后续如果只想改 `lookup("Ever", ...)` 这种上下文判别逻辑，也必须翻到同一个 persistence/mutation 文件；反过来如果只是改 load/save 或 add_alias，也会把 query heuristics 一起带进 diff。

# 主要目标

把 Rust registry 的 lookup/query 读面继续拆出去，同时保持外部 `crate::registry::*` surface 不变：

- `EntityRegistry::lookup()` 继续可用
- `EntityRegistry::extract_people_from_query()` 继续可用
- `EntityRegistry::extract_unknown_candidates()` 继续可用
- 调用方不需要改 import 路径

# 改动概览

这次新增了一个内部文件：

- `rust/src/registry_lookup.rs`

并把 `rust/src/registry.rs` 收回到 persistence / seed / mutation / summary 这类职责。

## 1. `registry_lookup`

这里现在承接：

- `lookup()`
- `extract_people_from_query()`
- `extract_unknown_candidates()`
- ambiguous-name 的 `disambiguate()`
- query-side context pattern 常量
- `regex_matches()` helper

也就是 registry 里“偏读面、偏启发式判别”的那部分。

## 2. `registry`

这里现在继续承接：

- `empty()`
- `load()` / `save()`
- `seed()` / `bootstrap()`
- `learn()`
- `add_person()` / `add_project()` / `add_alias()`
- `research()` / `confirm_research()`
- `summary()`
- `recompute_ambiguous_flags()` / `mode_context()`

也就是 registry 里“偏状态持久化、world model mutation”的那部分。

# 关键知识

## 1. registry 读面和写面变化节奏不同

读面 heuristics 更容易因为：

- 上下文模式
- ambiguous word 判别
- query extraction 规则

而频繁调整。

写面/persistence 更容易因为：

- bootstrap source
- save/load 格式
- alias / project / people mutation

而调整。

把这两类逻辑混在一起，会让每次启发式小改动都污染 persistence diff。拆开之后，review 更容易聚焦。

## 2. `impl EntityRegistry` 可以天然按 concern 分布在多个文件

这次没有新造 facade 类型，也没有改调用方路径，而是继续利用 Rust 允许：

- 同一个类型
- 多个 `impl` block
- 分布在不同模块文件

所以 `EntityRegistry` 依然还是同一个外部类型，但内部能力已经按 concern 分层。

# 补充知识

## 为什么 `summary()` 还留在 `registry.rs`

虽然 `summary()` 从使用场景上也算读面，但它更接近：

- registry 当前状态的直接序列化展示

而不是基于上下文的 lookup/disambiguation heuristics。

所以这次让它继续跟 persistence/mutation 放在一起，会更贴近“当前 registry 快照”的语义，而不是 query heuristic 语义。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

验证通过，说明这次 registry lookup split 没有改变外部 `EntityRegistry` 用法，也没有破坏现有 registry/runtime/CLI/MCP 的行为。

# 未覆盖项

这次没有继续改：

- `registry_runtime.rs`
- `registry_cli.rs`
- `mcp_runtime_registry.rs`
- `convo_general.rs`

因为目标只是把 `registry.rs` 内部的 lookup/query heuristics 继续拆出去，而不是继续往 runtime 或 CLI 层扩散改动。
