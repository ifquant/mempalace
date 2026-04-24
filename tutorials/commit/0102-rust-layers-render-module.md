# 背景

前几轮我们已经把 Rust 里的多个“service 内嵌逻辑”逐步抽成独立模块，比如：

- `palace_graph`
- `knowledge_graph`
- `dedup`
- `repair`

继续往下看，`wake_up()` 和 `recall()` 这条线里还有一块很典型的 Python `layers.py` 逻辑仍然留在 `service.rs`：

- identity 文本读取
- L1 文本渲染
- L2 文本渲染

这些逻辑本质上是 layer 表现层的一部分，不是 service 编排本身。

# 主要目标

这一提交的目标是把 Rust 的 layer 文本渲染职责继续收进 `layers` 模块：

1. 把 `wake_up/recall` 依赖的 identity / L1 / L2 文本逻辑从 `service.rs` 提升进 `layers.rs`
2. 保持 CLI、MCP、`LayerStack` 和 service 外部行为不变
3. 让 Rust 的 `layers` 模块更接近 Python `layers.py` 的真实边界

# 改动概览

- 更新 [rust/src/layers.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/layers.rs)
  - 新增 `default_identity_text()`
  - 新增 `read_identity_text()`
  - 新增 `render_layer1()`
  - 新增 `render_layer2()`
  - 补了对应单测
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `wake_up()` 现在复用 `read_identity_text()`
  - `wake_up()` / `recall()` 现在复用 `render_layer1()` / `render_layer2()`
  - 删除 service 内部那几段 layer 渲染 helper
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 补充 `layers` 模块已包含共享渲染 helper

# 关键知识

## 1. facade 和 helper 最好放在同一个领域模块里

`layers.rs` 之前已经有高层 facade：

- `LayerStack`
- `layer0()`
- `layer1()`
- `wake_up()`
- `recall()`

但真正把 drawer 列表渲染成：

- `## L1 — ESSENTIAL STORY`
- `## L2 — ON-DEMAND (...)`

的 helper 还留在 `service.rs`。  
这会造成一个边界问题：模块名叫 `layers`，但 layer 的具体文本规则却不在里面。

把 render helper 放回 `layers.rs` 后，模块职责会更一致：

- facade 在 `layers`
- 文本规则也在 `layers`

## 2. `service` 更适合保留数据获取，不适合保留表现层文本模板

这次拆分后，`service` 继续负责：

- 打开 SQLite
- 读取 recent drawers
- 构造 `SearchHit`
- 调用 count/token 相关逻辑

而“如何把这些数据拼成 L1/L2 文本”交给 `layers`。  
这是很典型的分层：

- service 负责拿数据
- 领域模块负责决定如何表达数据

# 补充知识

## 1. 共享 helper 抽出来后，测试通常更容易写

如果 `render_layer1()` 还埋在 `service.rs`，要测它通常得绕着 `App`、SQLite、palace path 走一圈。  
但抽出来之后，可以直接喂：

- 一个 `DrawerRecord`
- 一个 `SearchHit`

就测试文本输出是否符合预期。

这就是模块化的一个直接收益：测试粒度更细，失败时也更容易定位。

## 2. Python 对齐不只是“功能有了”，还包括“逻辑住对地方”

很多重写项目会先做到功能表面对齐，但内部边界还是旧的。  
这次做的事虽然不改 CLI 参数，也不改 MCP 返回，但它让 Rust 的代码组织更接近 Python：

- `layers` 模块不再只是壳
- 它开始真正拥有自己的文本规则

对长期维护来说，这类收口往往比再加一个小 flag 更值。

# 验证

已运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有改变 `wake_up`、`recall`、`layers-status` 的 CLI / MCP 外部行为
- 这次没有把 Layer 3 `search` 文本渲染挪进 `layers`，因为它已经有独立的 `searcher` 模块
- 这次没有改 Python `layers.py`
- 这次也没有动 `hooks/`、`docs/`、`.github/` 或其它仓库
