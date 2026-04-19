# 背景

前面已经把 Rust registry 这条线拆出过：

- `registry_types`
- `registry_lookup`
- `registry_research`

但 `rust/src/registry.rs` 里仍然混着两类不同职责：

- IO / bootstrap / summary
- mutation / confirm / ambiguous flag 维护

这意味着虽然 lookup 和 research 已经出去了，`registry.rs` 本身还是同时背着“文件持久化”和“状态变更”两类逻辑，后续继续演化时仍然容易变大。

# 主要目标

这次提交的目标是继续把 `registry.rs` 内部收紧成：

- IO / bootstrap 一层
- mutation 一层
- `registry` 保留 facade

同时保持外部 `crate::registry::*` surface 不变，不让调用方跟着迁移。

# 改动概览

这次新增了两个内部模块：

- `rust/src/registry_io.rs`
- `rust/src/registry_mutation.rs`

拆分后的职责边界是：

## `registry_io`

负责：

- `empty()`
- `load()`
- `save()`
- `seed()`
- `bootstrap()`
- `summary()`
- `research()`

这里主要是“registry 文件怎么读写”“onboarding/bootstrap 怎么灌初始数据”“summary 怎么组装”“research cache 怎么进出”。

## `registry_mutation`

负责：

- `learn()`
- `add_person()`
- `add_project()`
- `add_alias()`
- `confirm_research()`
- `recompute_ambiguous_flags()`

这里主要是“registry 状态怎么变化”。

## `registry`

现在只保留：

- public re-export
- 内部子模块声明
- registry 相关单测入口

这样外层仍然从 `crate::registry::*` 拿能力，但 `registry.rs` 本身已经不再塞满具体实现。

# 关键知识

## 1. 继续拆时，优先沿已有模块边界往下切

这次不是重新发明一套新结构，而是顺着前面已经形成的 registry family 往下切：

- types
- lookup
- research
- io
- mutation

这种“沿既有边界继续细化”的方式，比突然横向改成另一套完全不同的组织方式更稳，也更容易让后来的维护者看懂演进路径。

## 2. IO 和 mutation 分开，能减少语义噪声

一个很常见的坏味道是：

- 同一个文件既在做 load/save
- 又在做业务状态更新

这两类代码经常会一起出现，但关注点不一样：

- IO 关心序列化、路径、文件内容
- mutation 关心状态规则和一致性

把它们拆开之后，后面无论是改 onboarding bootstrap，还是改 alias/confirm 规则，定位都会更直接。

# 补充知识

## 1. facade 文件可以很薄，但测试锚点仍然值得保留

这次 `registry.rs` 变薄之后，没有把测试删掉，而是保留成 facade 的测试锚点。

这样做有两个好处：

- 外部 surface 的回归仍然能从统一入口观察
- 维护者打开 `registry.rs` 时，仍然能快速看到这条能力线的基本行为被什么测试覆盖

## 2. 内部重构时，优先保住 import 路径稳定

这次外部依然是：

- `crate::registry::*`

而不是要求所有上层调用方改成分别 import `registry_io`、`registry_mutation`。

这种做法在持续收口阶段特别重要，因为它把“内部结构优化”和“外部接口迁移”两件事分开了，验证成本更可控。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

这次没有改这些内容：

- 没有改变 registry JSON 的外部格式
- 没有改变 lookup / query 语义
- 没有改变 Wikipedia research 行为
- 没有继续拆 `registry_lookup` 内部 heuristics
- 没有改 Python `entity_registry.py`
