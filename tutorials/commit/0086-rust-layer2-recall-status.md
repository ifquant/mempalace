# 背景

Python 版有一条非常明确的 `layers.py` 使用路径：

- `wake-up` 看 L0 + L1
- `recall` 看 Layer 2 的按 wing/room 召回
- `search` 看 Layer 3 的深搜索
- `status` 看整个 layer stack 的概况

Rust 版之前只覆盖了其中一部分：

- `wake-up` 已有
- `search` 已有

但 Layer 2 的 `recall` 和“整个 layer stack 状态”还没有正式 CLI。

这会让 Rust 版在“我不是要语义搜索，我只是想按房间拿几条现成记忆”这种场景下少了一层工具面。

# 主要目标

1. 新增 `recall` 命令，对齐 Python `Layer2.retrieve()`
2. 新增 `layers-status` 命令，对齐 Python `MemoryStack.status()`
3. 保持 JSON / `--human` 双输出
4. 尽量复用现有 SQLite drawer 数据，不额外引入新存储

# 改动概览

- 扩展 [rust/src/model.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/model.rs)
  - 新增 `RecallSummary`
  - 新增 `LayerStatusSummary`
- 扩展 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - 新增 `App::recall()`
  - 新增 `App::layer_status()`
  - 新增 `render_layer2()`，生成人类可读的 Layer 2 文本
- 扩展 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - 新增 `recall`
  - 新增 `layers-status`
  - 新增对应的 human/json 错误与摘要输出
- 扩展 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - 覆盖 help
  - 覆盖 `recall --human`
  - 覆盖 `layers-status --human`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 1. `recall` 和 `search` 不是一回事

`search` 依赖 embedding，相当于 Python 的 Layer 3：

- 有 query
- 有 similarity
- 是 semantic search

`recall` 更接近 Python 的 Layer 2：

- 不做语义检索
- 只是按 `wing/room` 从已经归档的 drawers 里拿内容
- 适合“我知道要看哪一片记忆，但不想跑向量搜索”

所以这两个命令应该共存，而不是拿其中一个硬替另一个。

## 2. Layer 2 可以直接建立在 SQLite 上

这次 `recall` 没走 LanceDB，也没重新 embed。

原因是它本来就是“按条件回想已经知道的区域”，而不是“在全局里找最像的内容”。  
直接读 SQLite 的 drawers：

- 成本更低
- 更稳定
- 也更符合 Python `Layer2.retrieve()` 的语义

## 3. `layers-status` 的价值不是“又一个 status”

仓库里本来已经有 palace 级 `status`：

- drawer 总数
- wing/room breakdown

但 `layers-status` 关心的是另一层：

- L0 identity 有没有
- L0 大概多少 token
- L1/L2/L3 的职责是什么

也就是说它是“memory stack 的状态”，不是“palace 存储状态”。

# 补充知识

1. 如果一个系统同时有“存储状态”和“运行层状态”，命令名应该分开。  
   否则用户会很自然地问：`status` 到底是在看数据库，还是在看 memory stack？

2. 做 Layer 2 文本输出时，尽量复用现有结果结构，而不是重新造一个专用 DTO。  
   这次直接复用了 `SearchHit` 作为 recall 结果项，减少了另一套几乎相同的字段定义。

# 验证

执行过：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

并特别验证了：

```bash
cargo test --test cli_integration cli_recall_
cargo test --test cli_integration cli_layers_status_
```

# 未覆盖项

- 这次没有新增 MCP 层的 recall / layers-status 工具
- `recall` 当前仍然是基于 SQLite drawer 顺序和过滤，不带语义排序
- `layers-status` 目前是 Rust 自己的 palace-local 版本，没有回退到 Python 的 `~/.mempalace/identity.txt`
