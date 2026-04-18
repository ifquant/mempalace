## 背景

Rust 版已经有完整的搜索主链路：

- service: `App::search()`
- CLI: `search`
- MCP: `mempalace_search`

但 Python 还有一个更明确的库层文件：`python/mempalace/searcher.py`。  
它同时提供两类东西：

- 程序化搜索入口
- Python 风格的人类可读结果渲染

Rust 这边之前的情况是：

- 程序化搜索在 `service.rs`
- 人类可读搜索格式只存在于 `main.rs`

也就是说，调用方如果不是走 CLI，就拿不到一份稳定的“Python 风格搜索文本输出”能力。

## 主要目标

- 给 Rust 新增 `searcher` 模块
- 提供 `Searcher` façade
- 提供 `render_search_human()`
- 让 CLI 的搜索 human 输出复用库层 renderer
- 把这层能力写进 README

## 改动概览

- 新增 [rust/src/searcher.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/searcher.rs)
  - `Searcher`
  - `Searcher::search()`
  - `render_search_human()`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `searcher` 模块
- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - `print_search_human()` 改为直接复用 `searcher::render_search_human()`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. “程序化搜索” 和 “人类可读渲染” 都应该是库层能力

如果 renderer 只存在 CLI 里，会有两个问题：

- 别的 Rust 调用方无法稳定复用
- CLI 和非 CLI 调用方更容易慢慢漂成两套格式

这次把 `render_search_human()` 提到 `searcher` 模块之后：

- CLI 继续能用
- 测试更容易锁格式
- 以后如果要做 TUI / GUI / agent-side local rendering，也可以直接复用

### 2. façade 不应该抢走 service 的职责

`Searcher` 这次没有重新实现搜索逻辑，而是复用 `App::search()`。  
它的职责是：

- 提供更清晰的搜索语义入口
- 提供与 Python `searcher.py` 对应的模块形状
- 统一 human renderer

这样 service 仍然是业务真相来源，façade 只负责更友好的对外表面。

## 补充知识

### 1. renderer 抽到库层后，CLI 最好立即改成复用它

如果只新增 `render_search_human()`，但 CLI 还保留自己那份老实现，仓库里立刻就又有两份格式源头了。  
所以这次顺手把 `main.rs` 的 `print_search_human()` 收成了一行代理调用。

### 2. façade 测试最好同时测“执行”和“格式”

这次新增测试分成两类：

- `searcher_facade_runs_programmatic_search`
  - 锁 façade 真的能查出 drawer
- `render_search_human_matches_python_style_blocks`
  - 锁文本格式继续长得像 Python

这样后面如果有人只改了结果 shape 或只改了 renderer，都会马上暴露。

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

- `Searcher::search()` 能程序化返回 `SearchResults`
- `render_search_human()` 能生成 Python 风格结果块
- CLI `search --human` 继续复用同一份 renderer

## 未覆盖项

- 这次没有把 no-palace / error 文本 renderer 一起迁到 `searcher` 模块，仍然保留在 CLI 层
- 这次没有改 MCP 的搜索返回 shape，因为它已经走结构化 JSON，不依赖 human renderer
- 这次没有修改 Python `searcher.py`，只是把 Rust 库层收成更接近它的形状
