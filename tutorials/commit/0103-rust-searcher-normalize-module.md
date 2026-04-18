# 背景

前几轮我们已经持续把 Rust 重写里原本塞在 `service.rs` 的领域逻辑往各自模块收：

- `palace_graph`
- `knowledge_graph`
- `dedup`
- `repair`
- `layers`

继续往下看，`search` 这条线也还有一个明显的边界问题：

- `Searcher` 模块已经存在
- `render_search_human()` 也已经在 `searcher.rs`
- 但 search 结果的规范化逻辑还留在 `service.rs`

具体包括：

- `normalize_search_hits()`
- `normalize_source_file()`
- similarity rounding
- 搜索结果稳定排序

这实际上已经是 Python `searcher.py` 的职责，不应该继续挂在 service 里。

# 主要目标

这一提交的目标是把 Rust 的 search 结果整理逻辑继续收回 `searcher` 模块：

1. 把 search result normalization 从 `service.rs` 提升进 `searcher.rs`
2. 保持 CLI / MCP / service 的外部行为不变
3. 让 `searcher` 模块真正拥有“查询结果如何变成 Python 风格输出”的职责

# 改动概览

- 更新 [rust/src/searcher.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/searcher.rs)
  - 新增 `normalize_search_hits()`
  - 新增 `normalize_source_file()`
  - 新增 `round_similarity()`
  - 新增 `compare_search_hits()`
  - 把原本在 `service.rs` 的两条 search normalization 单测迁到这里
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `search()` 继续调用 `normalize_search_hits()`
  - `recall()` 继续调用 `normalize_source_file()`
  - 删除 service 内部对应 helper
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 补充 `searcher` 模块已经拥有结果规范化职责

# 关键知识

## 1. facade 和 normalization 逻辑最好一起放

如果一个模块已经叫 `searcher`，并且已经负责：

- `Searcher::search()`
- `render_search_human()`

那 search 结果的后处理规则也应该归它：

- basename 规范化
- similarity rounding
- 结果排序

否则模块边界会变得很奇怪：

- `searcher` 负责展示
- `service` 却决定展示前的结果语义

把 normalization 收回 `searcher` 后，职责就更统一了。

## 2. “不改行为，只挪边界” 最稳的办法是连测试一起搬

这次没有重新发明新的搜索规则，而是把原来 service 里的测试一起迁过去：

- `normalize_search_hits_uses_python_style_similarity_and_basename`
- `normalize_search_hits_keeps_duplicate_files_as_separate_hits`

这样做的意义是：

- 行为断言保持不变
- 测试所有权跟着模块边界一起走
- 后面如果 `searcher` 再演进，也能直接在模块内部保护自己的契约

# 补充知识

## 1. basename normalization 不是小事

搜索命中里的 `source_file` 如果有时候是：

- `notes/plan.txt`

有时候又是：

- `/tmp/project/notes/plan.txt`

CLI 和 MCP 的用户体验会明显变差。  
所以 `normalize_source_file()` 看起来只是个小 helper，但它其实在维护一个稳定的“用户可见协议”。

## 2. 稳定排序能减少“伪回归”

搜索类功能如果没有明确排序规则，很容易出现：

- 功能没坏
- 但测试或用户看起来像变了

这次把排序逻辑明确留在 `searcher` 模块里，本质上是在给搜索结果定义一个稳定协议。  
对 agent 协作和回归测试都很重要。

# 验证

已运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有改变 `search` CLI / MCP 的字段 shape
- 这次没有改变向量搜索本身的召回逻辑
- 这次没有把 `recall` 的排序逻辑也一并抽成新的 helper，因为当前还只是复用 `normalize_source_file()`
- 这次没有改 Python `searcher.py`
- 这次也没有动 `hooks/`、`docs/`、`.github/` 或其它仓库
