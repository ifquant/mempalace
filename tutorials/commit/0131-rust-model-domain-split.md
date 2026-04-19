# 背景

前几轮已经把 runtime、CLI、MCP、bootstrap、onboarding 等逻辑面逐步拆清，但 `rust/src/model.rs` 仍然把整个项目的 DTO、summary payload、request type 全堆在一个文件里。

随着 Rust rewrite 已经覆盖：

- palace read/write
- mining
- bootstrap / onboarding
- maintenance
- embedding runtime
- registry

这个单文件模型层开始变成一种“所有东西都往里塞”的中心点。后续如果只是改 registry 返回结构，也得翻一整个大文件；如果只是改 graph/search payload，也会和完全无关的 repair / onboarding / diary 类型混在一起。

# 主要目标

把 Rust 的模型定义继续按 domain 拆开，同时保持外部 API 不变：

- 继续保留 `crate::model::*` 这一层公共入口
- 不要求调用方改 import 路径
- 不改变现有 serde shape
- 不改变 CLI / MCP / service / storage 的行为

# 改动概览

这次新增了五个内部文件：

- `rust/src/model_palace.rs`
- `rust/src/model_ops.rs`
- `rust/src/model_project.rs`
- `rust/src/model_runtime.rs`
- `rust/src/model_registry.rs`

并把 `rust/src/model.rs` 收成一个只负责 re-export 的薄 facade。

## 1. `model_palace`

这里现在承接：

- `DrawerInput`
- `SearchHit`
- `CompressedDrawer`
- `SearchFilters`
- `SearchResults`
- `Status`
- `Rooms`
- `Taxonomy`
- graph traversal / tunnel / graph stats 相关类型

也就是和 palace read surface、search、graph、drawer 直接相关的模型。

## 2. `model_ops`

这里现在承接：

- `KgTriple`
- `KgFact`
- `KgQueryResult`
- `KgTimelineResult`
- `KgStats`
- `KgWriteResult`
- `KgInvalidateResult`
- `DiaryWriteResult`
- `DiaryEntry`
- `DiaryReadResult`
- `DrawerWriteResult`
- `DrawerDeleteResult`

也就是和 KG、diary、manual drawer write/read 相关的模型。

## 3. `model_project`

这里现在承接：

- `MineSummary`
- `MineRequest`
- `MineProgressEvent`
- `InitSummary`
- `OnboardingSummary`

也就是和 project bootstrap / mining / onboarding 相邻的模型。

## 4. `model_runtime`

这里现在承接：

- `MigrateSummary`
- `Repair*Summary`
- `Dedup*Summary`
- `DoctorSummary`
- `PrepareEmbeddingSummary`
- `CompressSummary`
- `WakeUpSummary`
- `RecallSummary`
- `LayerStatusSummary`

也就是运行期运维、embedding、compression、layer surface 的模型。

## 5. `model_registry`

这里现在承接：

- `RegistryLookupResult`
- `RegistrySummaryResult`
- `RegistryLearnResult`
- `RegistryWriteResult`
- `RegistryQueryResult`
- `RegistryResearchResult`
- `RegistryConfirmResult`

也就是 project-local entity registry 的结果 payload。

## 6. `model`

`model.rs` 现在不再自己定义所有 struct，而是：

- 用 `#[path = "..."]` 引入内部 domain 文件
- 统一 `pub use ...::*`

于是上层继续写 `use crate::model::...`，但内部实现已经按 domain 分层。

# 关键知识

## 1. 模型层也会成为“隐性大文件”

很多人会先拆 runtime 和 command dispatch，但 DTO / summary payload 往往会被默认继续堆在一个 `model.rs`。

问题是：当系统表面越来越多时，模型层会变成新的中心耦合点。

这种耦合不一定体现为逻辑 bug，但会体现在：

- diff 很杂
- review 很难扫
- 你很难一眼看出某个类型属于哪个 domain

所以当 rewrite 已经覆盖多个相对稳定的能力面时，把模型按 domain 收口是合理的下一步。

## 2. re-export facade 比让调用方改 import 更稳

这次没有让调用方改成：

- `crate::model_runtime::DoctorSummary`
- `crate::model_registry::RegistryQueryResult`

而是继续保留：

- `crate::model::DoctorSummary`
- `crate::model::RegistryQueryResult`

原因很直接：这里的重构目标是内部边界，不是给全仓引入一次 import churn。

做法是：

- domain 文件承接真实定义
- `model.rs` 继续作为公共 facade

这样内部可以继续演化，但上层 surface 保持稳。

# 补充知识

## 为什么这里用 `#[path = ...] mod ...`

因为当前仓库已经有一个现成的 `src/model.rs`，这次不需要顺手再把它改成 `src/model/mod.rs` 目录式结构。

所以这里采用的是更低扰动的办法：

- 保留 `model.rs`
- 在 `model.rs` 里用 `#[path = "..."]` 声明内部子模块
- 再统一 re-export

这样可以一次把 domain 分层做好，但避免把文件路径体系也一起翻掉。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

验证通过，说明这次模型层 domain split 没有改变现有 public surface，也没有破坏上层调用。

# 未覆盖项

这次没有继续改：

- `service.rs`
- `registry.rs`
- `palace_cli_*`
- `mcp_runtime_*`

因为目标只是把 `model.rs` 从“单大文件”收成 domain facade，而不是继续往 runtime 或 CLI 侧扩散改动。
