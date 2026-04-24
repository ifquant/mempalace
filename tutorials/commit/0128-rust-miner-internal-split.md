# 背景

前一轮已经把 `convo.rs` 按扫描、exchange、general 三层拆开了，但 `rust/src/miner.rs` 仍然同时承担三类职责：

- project/code/document mining
- convo/chat-export mining
- shared file discovery / chunking / drawer helper

这会让后续调整 mining 行为时，`project` 路径和 `convos` 路径继续绑在一个大文件里，改动边界不够清楚。

# 主要目标

把 Rust miner 内部继续按职责切开，同时保持外部 API 不变：

- `crate::miner::*` 继续是上层统一入口
- `service.rs` 不需要改调用方式
- 现有 tests 继续通过相同入口覆盖行为

# 改动概览

这次新增了三个内部模块：

- `rust/src/miner_project.rs`
- `rust/src/miner_convo.rs`
- `rust/src/miner_support.rs`

并把 `rust/src/miner.rs` 收成了一个薄 facade。

## 1. `miner_project`

这里现在承接：

- `mine_project_run()`
- project/file mining 主执行链
- room detection、project chunking、SQLite/LanceDB replace_source 写入

也就是“项目代码和文档挖掘”这条主链。

## 2. `miner_convo`

这里现在承接：

- `mine_conversations_run()`
- conversation normalize 后的 chunk 选择
- general/exchange 两条 convo ingest 路径
- convo drawer 写入和 dry-run/progress 分支

也就是“聊天记录挖掘”这条主链。

## 3. `miner_support`

这里现在承接：

- `chunk_text()`
- project file discovery
- file read helper
- `sanitize_slug()`
- default convo wing
- conversation drawer assembly helper

也就是 project/convo 两条主链共享的支持逻辑。

## 4. `miner`

这里现在只保留：

- `mine_project_run` re-export
- `mine_conversations_run` re-export
- `chunk_text` 的 crate-internal re-export

这样外部依然可以继续通过 `crate::miner` 使用 mining 入口，而不需要感知内部切分。

# 关键知识

## 1. mining facade 的价值在于稳住上层依赖

这次没有让 `service.rs` 或调用方直接改成依赖：

- `miner_project::*`
- `miner_convo::*`
- `miner_support::*`

而是继续让 `miner.rs` 充当 facade。这样之后如果还要继续细拆某一条 mining 链，不会让上层 import 路径继续变化。

## 2. project mining 和 convo mining 的变化节奏不同

这两条链虽然都叫 “mine”，但维护重心完全不同：

- project mining 更接近文件树扫描、chunking、room routing
- convo mining 更接近 normalize、extract mode、conversation drawer 写入

放在一个文件里时，任何一边的改动都会制造跨领域 diff。拆开之后：

- 调整 convo ingest 时不需要翻 project scanning 代码
- 调整 project mining 时也不会误碰 conversation drawer 语义

# 补充知识

## 为什么 `chunk_text()` 还继续通过 `miner` 对外暴露 crate-internal 入口

`service.rs` 里还有针对 `chunk_text()` 的内部测试路径，所以这次没有直接把它完全藏进 `miner_support`。

做法是：

- 真正实现挪到 `miner_support`
- `miner.rs` 继续做 crate-internal re-export

这样可以同时保住：

- 内部结构清晰
- 现有依赖面稳定

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

验证通过，说明这次 miner 内部分层没有改变 project/convo 两条 mining 主链的外部行为。

# 未覆盖项

这次没有继续改：

- `service.rs`
- `project_cli_mining.rs`
- `convo_*` 模块本身

因为目标只是把 `miner.rs` 的内部职责拆开，而不是继续上卷到 service 或 CLI 层。
