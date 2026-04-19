# 背景

前面已经把 registry 的 CLI、runtime、MCP surface 一层层拆出来了，但 `rust/src/registry.rs` 自己仍然很重：

- 一边定义 registry 数据结构
- 一边实现 `EntityRegistry` 行为
- 一边承担 Wikipedia lookup / classifier
- 一边放公共常量和 research helper

这会让后续继续调整 registry 行为时，很容易把“类型定义”和“外部 research 逻辑”一起拖进去。

# 主要目标

把 Rust registry 内部继续按职责切开，同时保持外部 API 基本不动：

- `crate::registry::*` 这条对外使用路径继续可用
- 行为面不变
- 测试和调用方不需要大面积跟着改

# 改动概览

这次新增两个内部模块：

- `rust/src/registry_types.rs`
- `rust/src/registry_research.rs`

并把 `rust/src/registry.rs` 收成更明确的 facade + behavior 层。

## 1. `registry_types`

这里现在承接：

- `COMMON_ENGLISH_WORDS`
- `RegistryPerson`
- `RegistryResearchEntry`
- `EntityRegistry`
- `RegistryLookupResult`
- `RegistrySummary`
- `RegistryLearnSummary`
- `SeedPerson`
- `RegistryLearnSummaryFields`

也就是 registry 的“数据定义面”。

## 2. `registry_research`

这里现在承接：

- `wikipedia_lookup()`
- `classify_wikipedia_summary()`
- summary truncation
- URL encoding
- 与 Wikipedia classifier 相关的测试

也就是 registry 的“外部 research / classifier 面”。

## 3. `registry`

这里现在只保留：

- `EntityRegistry` 的 load/save/seed/bootstrap/learn
- lookup / extract / disambiguation
- research / confirm_research 的 orchestration
- 对 `registry_types` 的 re-export

这样外部调用仍然可以继续写：

```rust
use mempalace_rs::registry::{EntityRegistry, SeedPerson, RegistryResearchEntry};
```

而不用感知内部拆分。

# 关键知识

## 1. facade 模块最重要的价值是“隔离内部重组”

这次没有让上层调用方直接改成依赖：

- `registry_types::*`
- `registry_research::*`

而是让 `registry.rs` 继续承担 facade 角色，统一 re-export。

这样以后即使还要继续拆 registry 内部：

- runtime
- CLI
- onboarding
- spellcheck

这些已经存在的调用点也不需要再跟着改路径。

## 2. “研究逻辑”和“状态机逻辑”最好分开

`EntityRegistry` 自己负责的是：

- 维护本地状态
- 做歧义判定
- 做 learn / confirm

而 Wikipedia lookup/classifier 负责的是：

- 访问外部来源
- 把页面摘要映射成 `person/place/concept/ambiguous`

这两层变化节奏不同，放在一个文件里会让后面的维护成本偏高。拆开之后：

- 研究逻辑可以单独演进
- registry 本体依然只关心“拿到研究结果后怎么落地”

# 补充知识

## 为什么这次没有继续把 disambiguation 再拆出去

`lookup()`、`extract_people_from_query()`、`disambiguate()` 这几块虽然也能继续拆，但它们仍然都属于 `EntityRegistry` 的核心行为面，彼此耦合很强。

所以这次先把最明显的两块外延职责拆走：

- 类型定义
- 外部 research

先把 `registry.rs` 的重心收回到 registry 本体，再决定是否继续细拆内部 heuristics。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

验证通过，说明这次内部切分没有改变 registry 相关的外部行为。

# 未覆盖项

这次没有修改：

- `registry_runtime`
- `registry_cli`
- MCP registry tools

因为目标只是把 `registry.rs` 的内部职责边界收紧，而不是改 registry 的对外 surface。
