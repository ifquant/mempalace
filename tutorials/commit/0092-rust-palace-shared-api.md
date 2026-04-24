## 背景

Python 版有一个很小但很关键的文件：`python/mempalace/palace.py`。  
它不是 CLI，也不是 MCP，而是把“共享 palace 操作”收在一起，供 miner 和其它入口复用：

- 默认 skip dirs
- 打开 collection
- `file_already_mined()`

Rust 这边虽然功能已经很多，但这层共享 API 还比较散：

- skip-dir 列表埋在 `service.rs`
- vector bootstrap 分散在 `init/init_project`
- project mining 和 convo mining 各自维护一套“文件有没有变化”的判断

这会让后面继续做库层复用、脚本调用、或更多入口接入时，越来越容易复制逻辑。

## 主要目标

- 给 Rust 新增共享 `palace` 模块
- 把默认 skip-dir 策略提升成显式仓库事实
- 提供 Rust 版 `file_already_mined()` / source-state helper
- 让 project mining 和 convo mining 复用同一套 unchanged-file 判断
- 补回归并同步 README

## 改动概览

- 新增 [rust/src/palace.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/palace.rs)
  - `SKIP_DIRS`
  - `ensure_vector_store()`
  - `file_already_mined()`
  - `source_state_matches()`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `palace` 模块
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `init/init_project` 改用 `ensure_vector_store()`
  - project mining 改用 `source_state_matches()`
  - convo mining 也改用 `source_state_matches()`
  - 不再在 `service.rs` 里自己维护一份 `SKIP_DIRS`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. “共享 helper 模块”不是为了好看，是为了减少语义漂移

这次真正要避免的是这种情况：

- project miner 改了一次 unchanged-file 逻辑
- convo miner 忘了同步
- 两条 ingest 路径慢慢长成两套语义

把判断收进 `palace::source_state_matches()` 之后，后面如果再改：

- mtime 优先级
- hash fallback
- source state 兼容策略

只需要改一处。

### 2. Python 风格 `file_already_mined()` 和 Rust 内部 `source_state_matches()` 不能混成一个概念

这次刻意保留了两个层级：

- `file_already_mined()`：
  - 更像 Python `palace.py`
  - 语义简单，适合脚本/库调用
- `source_state_matches()`：
  - 给 Rust miner 内部用
  - 支持 `mtime` 不可用时退回 `content_hash`

如果把这两个目的强行揉成一个函数，API 名字会变模糊，后面调用者也更容易误用。

## 补充知识

### 1. Rust 里“公共函数未使用”不一定是坏味道

像 `file_already_mined()` 这种函数，这次没有强行塞进所有内部调用路径。  
原因是它面向的是“库层对外语义”，而不是 miner 内部最完整的 source-state 判断。

只要：

- 名字清楚
- 测试覆盖到
- README 写明用途

这种 public helper 保留出来是合理的。

### 2. 小型 façade 模块可以先从常量和 helper 开始

很多人一说 “facade / shared API” 就想立刻上一个大 struct。  
其实最稳的第一步往往只是：

- 把共享常量挪出来
- 把重复判断封成函数
- 让两三个真实调用点先吃到它

这样既有价值，又不会过早设计一套太重的抽象。

## 验证

实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

本次新增/覆盖验证包括：

- `palace::file_already_mined()` 在 mtime 场景下的行为
- `palace::source_state_matches()` 在 hash fallback 场景下的行为
- project mining 和 convo mining 继续通过现有 unchanged-file / re-mine 回归

## 未覆盖项

- 这次没有把 `bootstrap.rs` 里的 `SKIP_DIRS` 也统一收进 `palace`，因为那条路径的目标是检测 bootstrap 候选文件，不完全等于 miner 语义
- 这次没有新增新的 CLI 或 MCP 命令，只补库层共享 API
- 这次没有改 Python `palace.py`，只是让 Rust 这边有了更明确的对应层
