# 背景

前几轮已经把 runtime、CLI、MCP、model、bootstrap、onboarding 等大块逐步拆清，但 `rust/src/service.rs` 还保留着一种典型的“总入口文件”形态：

- `App` 定义在这里
- 所有 `App` 方法实现也都堆在这里
- read / write / registry / maintenance / project mining 全混在一个 impl 里

虽然逻辑本身已经大量下沉到 runtime 模块，但 `service.rs` 还是承担着一个很重的“方法总汇”角色。后面如果只是改 registry 入口，也得翻到同一个大文件；如果只是改 palace read 入口，也会和 maintenance / diary / KG 混在一起。

# 主要目标

把 Rust 的 service-layer orchestration 继续按 capability family 拆开，同时保持外部 API 不变：

- 继续保留 `crate::service::App`
- 不改变 `App::new()` / `App::with_embedder()`
- 不改变各个 `App` 方法的签名
- 不要求 CLI / MCP / tests / library caller 改路径

# 改动概览

这次新增了五个内部文件：

- `rust/src/service_project.rs`
- `rust/src/service_read.rs`
- `rust/src/service_ops.rs`
- `rust/src/service_registry.rs`
- `rust/src/service_maintenance.rs`

并把 `rust/src/service.rs` 收成一个只保留 `App` 定义、构造函数和共享测试的薄 facade。

## 1. `service_project`

这里现在承接：

- `init()`
- `init_project()`
- `mine_project()`
- `mine_project_with_progress()`
- `compress()`

也就是 project/bootstrap/mining/compression 这条邻近链路。

## 2. `service_read`

这里现在承接：

- `status()`
- `list_wings()`
- `list_rooms()`
- `taxonomy()`
- `traverse_graph()`
- `find_tunnels()`
- `graph_stats()`
- `search()`
- `wake_up()`
- `recall()`
- `layer_status()`

也就是 palace read-side 和 layer/graph/search surface。

## 3. `service_ops`

这里现在承接：

- `add_kg_triple()`
- `query_kg()`
- `kg_query()`
- `kg_timeline()`
- `kg_stats()`
- `kg_add()`
- `kg_invalidate()`
- `add_drawer()`
- `delete_drawer()`
- `diary_write()`
- `diary_read()`

也就是 KG / diary / manual drawer 这组 write/read 操作。

## 4. `service_registry`

这里现在承接：

- `registry_summary()`
- `registry_lookup()`
- `registry_learn()`
- `registry_add_person()`
- `registry_add_project()`
- `registry_add_alias()`
- `registry_query()`
- `registry_research()`
- `registry_confirm_research()`

也就是 project-local entity registry 的整组入口。

## 5. `service_maintenance`

这里现在承接：

- `migrate()`
- `repair()`
- `repair_scan()`
- `repair_prune()`
- `repair_rebuild()`
- `dedup()`
- `doctor()`
- `prepare_embedding()`

也就是 maintenance 和 embedding runtime 相关入口。

## 6. `service`

`service.rs` 现在只保留：

- `App` struct
- `App::new()`
- `App::with_embedder()`
- 共享测试

于是 service facade 继续存在，但方法实现已经按 family 分流。

# 关键知识

## 1. 入口 facade 和实现分层可以同时成立

很多时候“把 service 拆掉”会被误解成“上层调用方也要一起改很多路径”。这次没有这样做。

这里的策略是：

- facade 继续保留：`crate::service::App`
- 不同 capability family 的 `impl App` 分散到不同文件

Rust 允许对同一个类型写多个 `impl` block，所以很适合做这种“外部 surface 不变，内部实现分层”的重构。

## 2. service 现在是 orchestrator，而不是方法仓库

前几轮已经把真正逻辑下沉到：

- `palace_read`
- `palace_ops`
- `registry_runtime`
- `maintenance_runtime`
- `init_runtime`
- `compression_runtime`
- `miner`

这意味着 `service` 最合理的定位，已经不再是“大而全逻辑层”，而是：

- 构造 `App`
- 提供稳定 façade
- 做必要的 init / runtime bridging

把它继续按 family 拆开后，这个边界更清楚了。

# 补充知识

## 为什么 `compress()` 放在 `service_project`

`compress` 也可以被理解成 maintenance，但在当前仓库语义里，它更接近：

- AAAK / wake-up / project memory surfacing

而不是 repair/migrate/dedup 这种运维修复动作。

所以这次把它和 `init / mine_project` 放在同一个 family，更贴近当前 repo 的功能分组。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

验证通过，说明这次 service family split 没有改变现有 `App` public surface，也没有破坏 CLI / MCP / tests 的调用。

# 未覆盖项

这次没有继续改：

- `registry.rs`
- `convo_general.rs`
- `palace_cli_*`
- `mcp_runtime_*`

因为目标只是把 `service.rs` 从“单一大 impl 文件”收成 capability-family facade，而不是继续把更下层业务模块一起拆动。
