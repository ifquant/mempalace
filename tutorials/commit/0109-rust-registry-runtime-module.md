# 背景

前几轮已经把 Rust 版的 `entity_registry` 能力做成了：

- `registry.rs`：数据结构、lookup、learn、research、confirm 等核心逻辑
- CLI / MCP：对外命令和工具表面

但 `service.rs` 里还保留着一大段 project-local registry orchestration：拼 `entity_registry.json` 路径、load/save、detect entities、把底层结果组装成 CLI/MCP 会消费的 summary。这样会让 `service` 继续承担“项目上下文 + 文件落盘 + 结果包装”这层职责。

# 主要目标

把 project-local registry orchestration 抽成独立 `registry_runtime` 模块，让：

- `registry.rs` 继续做纯 registry 逻辑
- `registry_runtime.rs` 负责项目路径、文件读写和 summary 组装
- `service.rs` 只保留 thin wrapper

# 改动概览

- 新增 [rust/src/registry_runtime.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/registry_runtime.rs)
- 提供 `RegistryRuntime`
- 收进去的能力包括：
  - `summary()`
  - `lookup()`
  - `learn()`
  - `add_person()`
  - `add_project()`
  - `add_alias()`
  - `query()`
  - `research()`
  - `confirm_research()`
- `service.rs` 里的 registry 入口现在统一委托给 `RegistryRuntime`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 为什么还需要 `registry_runtime`

`registry.rs` 现在已经很像一个“纯能力模块”，但它并不知道：

- 当前 project directory 是什么
- `entity_registry.json` 应该落在哪里
- learn 时应该从哪个项目目录重新检测 people/projects

这些都不是 registry 本体逻辑，而是“把 registry 放进项目上下文里如何运行”的问题。所以这里加一层 runtime/facade 是合理的。

## 这次没有把 `detect_entities_for_registry()` 再塞回 `registry.rs`

原因是 entity detection 现在已经有自己的模块边界：

- `entity_detector.rs`: 检测
- `registry.rs`: registry 数据与规则
- `registry_runtime.rs`: 项目级 orchestration

这样三层职责比较干净，不会重新把 detector 和 registry 耦在一起。

# 补充知识

很多时候模块化最容易漏掉的不是“算法函数”，而是这种中间层 orchestration。  
如果只抽纯函数，`service.rs` 最终还是会积累大量：

- 路径拼装
- load/save
- Result shape 转换

这轮的价值就在于把这层项目级 registry runtime 也显式化。

# 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

# 未覆盖项

- 这次没有改变 CLI / MCP registry 表面
- 这次没有改变 entity detection 规则
- 后续如果继续收口，`service.rs` 还可以再拆 KG write 面或 diary/write surfaces
