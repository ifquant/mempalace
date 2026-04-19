## 背景

前几轮已经把 `init`、`maintenance`、`palace_read`、`palace_ops`、`registry_runtime` 等大块 orchestration
从 `service.rs` 里拆了出来，但 `compress` 和 `doctor / prepare-embedding` 这条 runtime 路径还残留了一段：

- `compress` 虽然已有 `compress.rs` 负责压缩规划，但真正的 SQLite 打开、profile 校验、持久化仍然留在 `service.rs`
- `embedding_runtime.rs` 还是偏 helper 风格，`service.rs` 继续自己持有命令级 orchestration

这会让 `service.rs` 继续承担不必要的 runtime 细节，不符合这条 Rust rewrite 最近一贯的“service 只保留 thin wrapper，真实流程进独立模块”的收口方向。

## 主要目标

- 把 `compress` 的 runtime orchestration 从 `service.rs` 挪进独立模块
- 把 `embedding_runtime` 从 helper 集合提升成真正的 facade
- 继续缩小 `service.rs`，让它只负责把 `AppConfig` 和 embedder 委托给下游 runtime

## 改动概览

- 新增 `rust/src/compression_runtime.rs`
  - 提供 `CompressionRuntime`
  - 收口 `compress` 需要的 SQLite 打开、schema/profile 校验、读取 drawer、可选持久化
- 扩展 `rust/src/embedding_runtime.rs`
  - 新增 `EmbeddingRuntime { config, embedder }`
  - 提供 `doctor()` 和 `prepare_embedding()` facade
  - 保留原来的 `EmbeddingRuntimeContext`、`finalize_doctor_summary()`、`prepare_embedding_run()` 作为模块内共享 helper
- 更新 `rust/src/service.rs`
  - `doctor()` 现在直接委托给 `EmbeddingRuntime`
  - `prepare_embedding()` 现在直接委托给 `EmbeddingRuntime`
  - `compress()` 现在直接委托给 `CompressionRuntime`
  - 删掉这一轮不再需要的本地导入
- 更新 `rust/src/lib.rs`
  - 导出 `compression_runtime`
- 更新 `rust/README.md`
  - 明确说明 `compression_runtime` 和 `EmbeddingRuntime` facade 已经成为 Rust library structure 的一部分

## 关键知识

### 为什么 `compress.rs` 和 `compression_runtime.rs` 要分开

这里延续的是前几轮已经形成的模块边界：

- `compress.rs` 负责“纯压缩规划”
- `compression_runtime.rs` 负责“打开存储、读取数据、决定是否写回”

这样做的好处是：

- 压缩规则本身可以单独复用和测试
- CLI / MCP / 未来库调用如果都要跑压缩，不需要再各自复制一遍 SQLite orchestration
- `service.rs` 不会重新长回去

### 为什么 `embedding_runtime` 还保留 helper

`EmbeddingRuntime` 变成 facade 以后，`finalize_doctor_summary()` 和 `prepare_embedding_run()` 仍然值得保留成模块级 helper，因为：

- `doctor` 的 summary 补全逻辑是稳定的、可测试的
- `prepare-embedding` 的 warmup retry loop 和最终 summary 组装是相对独立的子过程

也就是说，这一轮不是简单地“全塞回 struct impl”，而是把模块同时做成：

- 对外有 facade
- 对内保留可复用 helper

## 补充知识

这轮之后 `service.rs` 的职责更加统一了：

- 持有 `AppConfig`
- 持有 embedder
- 作为应用入口暴露稳定 API
- 把真正的 orchestration 分发给：
  - `init_runtime`
  - `maintenance_runtime`
  - `palace_read`
  - `palace_ops`
  - `registry_runtime`
  - `miner`
  - `compression_runtime`
  - `embedding_runtime`

这种结构更接近 Python 里“按功能文件分开”的组织方式，只是 Rust 这边再额外强化了一层 runtime facade。

## 验证

本轮执行并通过：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这轮没有继续拆 `mcp.rs`；它仍然是下一类“大文件但表面稳定”的候选收口点
- `CompressionRuntime` 目前只覆盖 `compress`；如果以后出现 AAAK 相关的更多 runtime 入口，再考虑进一步合并
- `EmbeddingRuntime` 目前只覆盖 `doctor / prepare-embedding`，没有顺手扩展到其它命令
