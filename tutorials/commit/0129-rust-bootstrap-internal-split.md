# 背景

前几轮已经把 CLI、registry、convo、miner 等大模块逐步收成了更清楚的结构，但 `rust/src/bootstrap.rs` 里仍然同时承担三种职责：

- project bootstrap orchestration
- config/entities 文件读写
- registry / AAAK / critical facts 文档模板生成

这会让后续只想调整 bootstrap 文档模板时，也必须翻同一个 orchestration 文件；反过来如果只是改 bootstrap 编排，也会把一大坨模板输出逻辑一起带进 diff。

# 主要目标

把 Rust bootstrap 内部继续按职责切开，同时保持外部 API 不变：

- `bootstrap_project()` 入口不变
- `default_wing()` 和 `write_project_config_from_names()` 继续可用
- `init_runtime`、`onboarding`、CLI 等调用方不需要大面积调整

# 改动概览

这次新增了两个内部模块：

- `rust/src/bootstrap_files.rs`
- `rust/src/bootstrap_docs.rs`

并把 `rust/src/bootstrap.rs` 收成 orchestration 层。

## 1. `bootstrap_files`

这里现在承接：

- 已有 `mempalace.yaml` 的读取
- 新 `mempalace.yaml` 的写入
- `entities.json` 的读取和写入
- `write_project_config_from_names()`

也就是 bootstrap 里“和普通结构化文件打交道”的那部分。

## 2. `bootstrap_docs`

这里现在承接：

- `write_entity_registry()`
- `write_aaak_entities()`
- `write_critical_facts()`

也就是 bootstrap 里“生成世界建模文档和 registry 文件”的那部分。

## 3. `bootstrap`

这里现在只保留：

- `InitBootstrap`
- `bootstrap_project()`
- `default_wing()`
- 对 `write_project_config_from_names()` 的继续暴露
- orchestration 相关测试

这样 `bootstrap.rs` 的重心重新回到“如何组织 bootstrap 过程”，而不是继续夹带所有文件模板细节。

# 关键知识

## 1. bootstrap orchestration 和 file/doc generation 变化节奏不同

这两层看起来都属于 init/bootstrap，但维护节奏不一样：

- orchestration 关心的是“什么时候写、什么时候保留已有文件”
- docs/files helper 关心的是“每个文件长什么样”

把它们放在一个文件里，任何模板小改动都会制造和 orchestration 混在一起的大 diff。拆开之后，边界更清楚。

## 2. re-export 比强迫上层改路径更稳

这次仍然保留了：

- `bootstrap_project()`
- `default_wing()`
- `write_project_config_from_names()`

通过 `bootstrap.rs` 继续对外提供，而不是让调用方直接改去依赖 `bootstrap_files`。这样以后如果还要继续细拆 bootstrap 内部，不会把上层 import 路径也反复改来改去。

# 补充知识

## 为什么 `write_project_config_from_names()` 继续挂在 `bootstrap`

这个 helper 虽然实现已经挪到 `bootstrap_files`，但语义上仍然更像 bootstrap 的一部分，而且调用方更容易自然地从 `bootstrap` 去找它。

所以这次做法是：

- 真正实现下沉到 `bootstrap_files`
- `bootstrap.rs` 继续对外暴露它

这样既能收紧内部结构，也能保持对外接口稳定。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

验证通过，说明这次 bootstrap 内部分层没有改变现有 init/bootstrap 的外部行为。

# 未覆盖项

这次没有继续改：

- `init_runtime`
- `project_cli_bootstrap`
- `onboarding`

因为目标只是把 `bootstrap.rs` 的内部职责拆开，而不是继续往上卷到 runtime 或 CLI 层。
