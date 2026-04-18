# 背景

前几轮我们一直在做同一类收口：把 Rust 重写里原本塞在 `service.rs` 的领域逻辑，逐步送回各自的模块。

已经拆出来的包括：

- `repair`
- `layers`
- `searcher`
- `drawers`
- `compress`

继续往下看，`doctor` 和 `prepare_embedding` 这条 embedding runtime 路径也还留着一块典型的 service 内嵌逻辑：

- doctor summary 的路径/version 回填
- prepare warmup 的 retry loop
- prepare result 的 summary 组装

这些逻辑虽然不复杂，但它们已经形成了一条稳定的小边界，不应该一直挂在 `service.rs` 里。

# 主要目标

这一提交的目标是把 Rust 的 embedding runtime 这条线收成独立模块：

1. 新增 `embedding_runtime` 模块
2. 把 doctor summary finalize 和 prepare retry/summary 从 `service.rs` 挪出去
3. 保持 `doctor` / `prepare-embedding` 的 CLI、MCP、service 外部行为不变

# 改动概览

- 新增 [rust/src/embedding_runtime.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/embedding_runtime.rs)
  - `EmbeddingRuntimeContext`
  - `finalize_doctor_summary()`
  - `PrepareEmbeddingRun`
  - `prepare_embedding_run()`
  - 补了 hash embedder 成功 warmup 的单测
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `embedding_runtime`
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `doctor()` 现在复用 `finalize_doctor_summary()`
  - `prepare_embedding()` 现在复用 `prepare_embedding_run()` 和 `PrepareEmbeddingRun::into_summary()`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 补充 `embedding_runtime` 模块说明

# 关键知识

## 1. “retry loop + summary builder” 本身就是一个稳定边界

`prepare_embedding` 之前虽然只是几十行，但里面实际包含了两层职责：

- 运行 warmup 重试
- 把运行结果变成最终 summary

这类逻辑很容易在 service 里越长越乱。  
把它抽出来之后，service 就只需要：

- 准备 context
- 调模块
- 返回结果

## 2. doctor 和 prepare 其实共享同一套运行时上下文

这次用 `EmbeddingRuntimeContext` 收了几类稳定字段：

- `palace_path`
- `sqlite_path`
- `lance_path`
- `version`
- `provider`
- `model`

这套字段在 doctor / prepare 两个命令里几乎是一模一样的。  
单独用 context 统一，能避免 service 以后再次复制粘贴这批字段。

# 补充知识

## 1. 抽模块时，先抽“可重复出现的协议”，不要先抽最底层依赖

这次没有试图把 embedder trait 或 fastembed/hash backend 再做一层抽象。  
因为真正重复出现的不是 backend 本身，而是：

- doctor summary 回填协议
- prepare retry 协议

优先抽“协议”通常比优先抽“依赖”更稳。

## 2. 单测 hash backend 是这类模块最稳的入口

`prepare_embedding_run()` 需要真实 embedder，但拿 fastembed 去测会把测试变重。  
这里直接用 hash backend 做单测，能覆盖：

- retry loop 的主路径
- doctor/warmup summary 的组合方式

这是在重写仓库里很常见的策略：  
用最便宜的 backend 锁住 orchestration 行为。

# 验证

已运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有改变 fastembed/hash backend 的实现
- 这次没有改变 `doctor` / `prepare-embedding` 的 CLI 或 MCP 参数
- 这次没有改变 human 输出格式
- 这次没有改 Python `dialect.py` / `layers.py` / CLI
- 这次也没有动 `hooks/`、`docs/`、`.github/` 或其它仓库
