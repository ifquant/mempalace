# 背景

Rust 版 MemPalace 之前已经把很多大文件按能力边界拆开了，但 `rust/src/embed.rs` 还同时背着三类完全不同的职责：

- 本地 hash embedding provider
- `fastembed` provider 和模型 warm-up / doctor 行为
- ONNX Runtime、`HF_ENDPOINT`、模型缓存路径这些运行时环境 helper

这种结构还能工作，但文件继续长下去后，会让 embedding 这条线重新变成一个新的“超级模块”。

# 主要目标

这次提交的目标不是改 embedding 行为，而是把 `embed.rs` 的内部职责继续收紧成几个自然模块，同时保持外部 API 不变。

也就是说，仓库里的其它调用方仍然通过 `crate::embed::*` 使用 embedding 层，但 `embed` 自己不再同时承载所有实现细节。

# 改动概览

这次改动主要包括四部分：

1. 把 `rust/src/embed.rs` 改成薄 facade。
2. 新增 `rust/src/embed_hash.rs`，只放 hash provider。
3. 新增 `rust/src/embed_fastembed.rs`，只放 fastembed provider。
4. 新增 `rust/src/embed_runtime_env.rs`，只放运行时环境和缓存 helper。

拆分后的职责边界是：

- `embed_hash`
  - `HashEmbedder`
  - token hashing
  - hash doctor summary
- `embed_fastembed`
  - `FastEmbedder`
  - model init
  - warm-up
  - doctor summary
- `embed_runtime_env`
  - `configure_ort_dylib_path()`
  - `detect_ort_dylib_path()`
  - `configure_hf_endpoint()`
  - `model_cache_ready()`
  - `expected_model_file()`
- `embed`
  - `EmbeddingProfile`
  - `EmbeddingProvider`
  - `build_embedder()`
  - 对外 re-export

这样做之后，上层代码仍然只依赖 `embed` 这个统一入口，但实现细节已经按 provider 和 runtime helper 分开。

# 关键知识

## 1. Rust 里可以用 facade 模块保留稳定入口

这次拆分没有把外部调用方改成直接 import `embed_hash` 或 `embed_fastembed`，而是让 `embed.rs` 保留统一入口：

- 对外继续暴露 trait 和 profile
- 对内用 `#[path = ...] mod ...;` 组织子模块
- 再通过 `pub use` / `pub(crate) use` 把需要的符号重新导出

这种做法的好处是：

- 外部 API 比较稳定
- 内部实现可以继续演化
- 调用方不会因为一次内部重构被迫大面积改 import

## 2. `pub use` 和 `pub(crate) use` 的边界不同

这次拆分里有两种 re-export：

- `pub use`
  - 给 crate 外部或上层模块稳定使用
- `pub(crate) use`
  - 只给当前 crate 内部共享

例如 `embed_runtime_env` 里的 helper 不需要成为公开 API，所以这里只做 `pub(crate) use`。

这是一种很常见的 Rust 分层手法：对外只暴露真正稳定的 surface，对内再共享实现细节。

# 补充知识

## 1. 模块拆分优先按“职责变化速度”来切

不是所有大文件都要机械地平均拆分。更实用的做法是看里面有没有几类“变化原因完全不同”的代码。

这次 `embed` 就很典型：

- hash provider 是纯本地算法逻辑
- fastembed provider 会跟模型、runtime、下载行为一起变化
- runtime env helper 会跟本机环境和 cache layout 一起变化

既然变化原因不同，就应该尽量拆开。

## 2. 做内部重构时，先保住对外 surface，风险会小很多

如果一边拆内部实现，一边顺手改 public API，就很难判断问题到底来自：

- 行为变化
- 接口变化
- 调用方迁移不完整

这次先只做 internal split，不改外部 embedding surface，所以验证成本会更可控。

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

- 没有改变 hash / fastembed 的行为语义
- 没有改变 `doctor` / `prepare-embedding` 的 CLI 或 MCP 表面
- 没有改 embedding provider 的默认选择策略
- 没有继续往更细粒度拆 `fastembed` 内部，例如把 doctor/warm-up 再拆成单独文件
