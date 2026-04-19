# 背景

前一轮已经把 Rust 的 project bootstrap CLI 拆成：

- `project_cli_bootstrap_init`
- `project_cli_bootstrap_onboarding`
- `project_cli_bootstrap_support`

紧邻的 `rust/src/project_cli_mining.rs` 仍然同时装着两条不同节奏的命令线：

- `mine`
- `search`

并且还混着：

- handler
- human/json renderer
- mine progress callback
- search 的 no-palace 分支
- shared config/app/json helper

这会让任何一次 mining/search 相关的小改动，都把另一条命令线一起拖进 diff。

# 主要目标

把 Rust project mining CLI 再按命令族切开，同时保持外部 surface 不变：

- `handle_mine()` 继续从 `project_cli_mining` 暴露
- `handle_search()` 继续从 `project_cli_mining` 暴露
- `project_cli.rs` 和顶层 CLI dispatch 不需要改调用方式
- 用户可见的 mine/search 行为、输出和错误路径不变化

# 改动概览

这次新增了三个内部文件：

- `rust/src/project_cli_mining_mine.rs`
- `rust/src/project_cli_mining_search.rs`
- `rust/src/project_cli_mining_support.rs`

并把 `rust/src/project_cli_mining.rs` 收成一个薄 facade。

## 1. `project_cli_mining_mine`

这里现在承接：

- `handle_mine()`
- mine 的 progress callback
- mine 的 human 输出
- mine 的 JSON error 输出

也就是 `mine` 这条命令线自己的 CLI 面。

## 2. `project_cli_mining_search`

这里现在承接：

- `handle_search()`
- search 的 no-palace 分支
- search 的 human 输出
- search 的 JSON error 输出

`search` 虽然也属于 project-facing CLI，但它的运行前提和错误面跟 `mine` 不同，单独拆出去之后边界更清楚。

## 3. `project_cli_mining_support`

这里现在承接 mining family 共享 helper：

- `resolve_mining_config()`
- `create_mining_app()`
- `print_mining_json()`

这样 mining family 自己有一层稳定的 support surface，不再继续和更大的 project helper 混在一起。

## 4. `project_cli_mining`

这个文件现在只保留 re-export：

- `handle_mine()`
- `handle_search()`

它的职责变成“mining command family 的薄入口”，不再承载具体实现。

# 关键知识

## 1. `mine` 和 `search` 的维护节奏不一样

虽然它们都在 project CLI 里，但变化来源不同：

- `mine` 更容易因为 ingest mode、progress、dry-run、include-ignored 而调整
- `search` 更容易因为 query error、no-palace、human result view 而调整

拆开之后，任何一侧的小改动都不会再无谓地把另一侧拖进 review。

## 2. progress callback 应该跟命令线走

`mine` 里最容易变复杂的地方之一，不是 `MineRequest` 本身，而是：

- dry-run progress
- filed progress
- stderr 输出格式

这部分本质上就是 `mine` command contract 的一部分，所以这次和 `handle_mine()`、`print_mine_human()` 一起留在同一个文件更合适。

# 补充知识

## 为什么 `search` 的 no-palace 逻辑不抽成更通用 helper

仓库里已经有通用的 `print_no_palace()`，但 search 还额外有自己的 human 文本和退出时机。把它继续放在 `project_cli_mining_search` 里，能避免为了少几行代码把 search-specific contract 过早抬升成全局 helper。

## facade re-export 对这种连续收口特别有效

这几轮 CLI 收口一直沿用同一个套路：

- 真实实现继续细分
- 原 family 模块只做 `pub use`

它的好处不是“技巧感”，而是能稳定上层 import，让你可以连续多轮把内部结构收紧，而不会把外层 API 路径打断。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

这些检查通过后，可以确认：

- mining CLI split 没破坏现有命令 surface
- 新的 facade + support 结构没有打断编译
- 现有 CLI / service / MCP 回归仍然保持绿色

# 未覆盖项

这次没有继续改：

- `project_cli_transcript.rs`
- `normalize.rs`
- `searcher.rs`
- `miner.rs`

因为目标只是把 project mining CLI 再按命令族切开，而不是继续扩散到 transcript family 或更底层的 mining/search library 模块。
