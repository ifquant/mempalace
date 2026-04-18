## 背景

Rust 版前面已经把这些高层能力补出来了：

- `wake-up`
- `recall`
- `search`
- `layers-status`

而且 CLI 和 MCP 都能调它们。  
但从“库层可编程 API”来看，Rust 还缺 Python `layers.py` 里那种明确的统一入口：

- Layer 0
- Layer 1
- Layer 2
- Layer 3

如果调用方只是想在 Rust 里嵌入 MemPalace，当下只能直接碰 `App` 和一堆 service 方法，缺少一个面向“memory stack”概念的清晰 façade。

## 主要目标

- 给 Rust 新增 `layers` 模块
- 提供程序化 `LayerStack`
- 提供显式的 `layer0()` / `layer1()`
- 复用现有 `wake_up()` / `recall()` / `search()` / `status()`
- 把这块库层能力写进 README

## 改动概览

- 新增 [rust/src/layers.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/layers.rs)
  - `LayerStack`
  - `Layer0State`
  - `Layer1State`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `layers` 模块
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 把 `LayerStack` 标成仓库事实

`LayerStack` 当前提供：

- `layer0()`
- `layer1()`
- `wake_up()`
- `recall()`
- `search()`
- `status()`

## 关键知识

### 1. façade 的价值是“按概念组织 API”，不是重复实现

这次没有再重写一套 layer 逻辑。  
`LayerStack` 做的事很克制：

- `layer0()` / `layer1()` 用现有 `wake_up()` 结果裁出子视图
- `wake_up()` / `recall()` / `search()` / `status()` 直接复用 `App`

也就是说，这层 façade 主要解决的是：

- API 可发现性
- 概念分组
- 调用体验

而不是引入第二套实现。

### 2. Layer 0 / Layer 1 单独暴露，比只留 wake-up 更接近 Python 心智

`wake_up()` 很适合 agent 实际启动时整包加载。  
但 Python `layers.py` 的价值之一是：它把 Layer 0 和 Layer 1 也当成独立概念存在。

这次 Rust 也补了这层区分：

- `layer0()` 取 identity 视图
- `layer1()` 取 essential story 视图

这样后续如果要做更细的 token-budget 管理，或者让宿主应用只拿某一层，也更自然。

## 补充知识

### 1. 抽 façade 时，最稳的是先从“已有稳定 service”往上包

如果直接在 façade 里自己重建 SQLite / LanceDB / embedder 细节，很容易长出第三套语义。  
这次选择继续经由 `App` 调现有 service，就是为了避免这一点。

### 2. “库 API 对齐”通常比 CLI 对齐更容易被忽略

很多迁移做到后面，CLI/MCP 都差不多了，但真正给其他代码调用的 API 还是散的。  
对一个重写项目来说，这类 façade 很重要，因为它决定了：

- 别的 Rust crate 怎么接它
- 测试/脚本怎么复用它
- 未来怎么把内部 service 稳定地暴露出去

## 验证

实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

本次新增覆盖了：

- `LayerStack::layer0()` 能读取 identity
- `LayerStack::layer1()` 能生成 Layer 1 文本
- 现有 `wake_up/recall/search/status` 继续通过已有 CLI / service 回归

## 未覆盖项

- 这次没有把 CLI 或 MCP 改成走 `LayerStack`，它们仍然通过现有 `App` service
- 这次没有新增 Layer 3 的额外 typed summary，因为 Rust 现有 `SearchResults` 已经足够承担那层返回
- 这次没有修改 Python `layers.py`，只是把 Rust 库层对齐到更接近它的形状
