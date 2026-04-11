# 背景

前一提交虽然把 Rust 版 MemPalace 的 CLI、MCP、存储和测试骨架搭起来了，但 embedding 仍然只是一个本地哈希近似方案。

那种做法适合：

- 打通流程
- 做离线测试
- 保证 CI 稳定

但不适合继续把 Rust 版往“真实替代 Python 版”推进，因为检索质量上限太低。

# 主要目标

这次提交做了 4 件更接近真实产品的事：

1. 把 embedding 从单函数实现改成 provider 抽象
2. 接入 `fastembed` 作为真实本地 embedding provider
3. 保留 `hash` provider 作为测试和离线保底路径
4. 用 SQLite `meta` 固化 embedding profile，避免不同向量维度静默混用

# 改动概览

主要代码调整：

- `rust/src/config.rs`
  - 新增 embedding 配置：provider、model、cache dir、download progress
  - 默认 provider 改为 `fastembed`
- `rust/src/embed.rs`
  - 新增 `EmbeddingProvider` trait
  - 新增 `HashEmbedder`
  - 新增 `FastEmbedder`
  - `FastEmbedder` 默认走 `MultilingualE5Small`
  - E5 模型会自动加 `query:` / `passage:` 前缀
- `rust/src/service.rs`
  - `App` 现在持有 provider
  - `mine` 改成 batch document embedding
  - `search` 改成 provider 驱动的 query embedding
- `rust/src/storage/sqlite.rs`
  - 新增 `embedding_provider / embedding_model / embedding_dimension` 元数据
  - 如果打开的是旧的 hash palace，会做 legacy hash 检查，阻止和 fastembed 混用
- `rust/src/storage/vector.rs`
  - 向量表 schema 不再写死 64 维，而是跟 provider 维度对齐
- `rust/tests/*.rs`
  - 测试显式切回 `hash` provider，避免 CI 依赖模型下载

环境与构建层：

- `rust/Cargo.toml`
  - 引入 `fastembed`
  - 改成 `ort-load-dynamic` 路线，不再依赖构建期下载 ONNX Runtime 二进制
- `rust/README.md`
  - 补充 embedding 配置和本地 runtime 说明

# 关键知识

## 1. 真正可维护的 embedding 接口应该先抽象 provider

如果一开始把 embedding 直接写死在 `mine` / `search` 里，后面每换一个模型、每加一个本地/远程方案，都要把业务层再拆一次。

更稳的做法是先抽一个 provider 边界：

- `profile()`
- `embed_documents()`
- `embed_query()`

这样 service 层只关心“我要向量”，不关心具体是 hash、fastembed，还是未来的别的本地模型。

## 2. 向量库和 embedding 维度必须绑定

这次新增的 SQLite `meta` 不是可有可无的装饰。

因为一旦：

- 旧 palace 用的是 64 维 hash
- 新 palace 用的是 384 维 fastembed

你又没有记录 profile，系统就可能把不同维度的数据混到同一 palace 里，最后在 search 阶段才炸，甚至更糟的是悄悄出错。

所以必须把：

- provider
- model
- dimension

持久化成仓库事实。

# 补充知识

## 为什么不用构建期自动下载 ONNX Runtime

一开始直觉上会觉得“让依赖自己下载最方便”，但实际工程里这种路径常见问题很多：

- CDN 不稳定
- TLS 栈兼容问题
- CI / 国内网络波动
- 首次构建时间过长

这次改成 `ort-load-dynamic` 后，编译不再依赖远程 ORT 二进制。  
运行时只需要系统里有动态库即可。

## 为什么测试继续用 hash provider

因为测试的目标不是验证 embedding 模型质量，而是验证：

- mine/search 流程
- palace schema
- CLI/MCP 协议面
- KG / taxonomy / ingest bookkeeping

如果把这些测试都绑死在真实模型下载上，CI 会变脆，反馈速度会显著变差。

所以这里保留 `hash` provider，不是倒退，而是把“工程稳定性”和“真实能力”拆开处理。

# 验证

已完成：

- `cd rust && cargo check`
- `cd rust && cargo test`
- `cd rust && cargo fmt --check`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`
- `brew install onnxruntime`

额外验证：

- 真实 `fastembed` 路径已跑到模型下载阶段
- 本机已安装 Homebrew `onnxruntime`
- 代码会自动探测 `/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib`

说明：

- 首次 `fastembed` 执行会拉取 HuggingFace 模型，速度受网络影响明显
- 本轮没有等待完整模型下载结束，因此没有把首次 fastembed mine/search 完整跑完

# 未覆盖项

这次仍然没有做：

- 把 `fastembed` 变成所有测试默认路径
- 对不同 fastembed 模型做 benchmark 对比
- 按模型自动生成更细的 query/document prompt 策略
- 远程 embedding provider
- Python 版语义检索质量对标

这次提交的定位很明确：  
Rust 版已经从“假 embedding 骨架”进入“真实本地 embedding 架构”，但真实模型效果和性能调优还在下一阶段。
