## 背景

上一轮已经把 registry 子命令从 `main.rs` 里抽成了 `registry_cli.rs`，但 palace-facing 的另一大块 CLI 仍然堆在主文件里：

- `compress`
- `wake-up`
- `recall`
- `layers-status`
- `migrate`
- `repair`
- `dedup`
- `status`
- `doctor`
- `prepare-embedding`

这组命令有一个共同点：都围绕 palace 本体、运维状态、层级读取或 embedding runtime 展开。继续把它们留在 `main.rs` 里，会让顶层分发文件继续承担太多主题内细节。

## 主要目标

- 把 palace-facing CLI 命令族从 `main.rs` 收成独立模块
- 让 `main.rs` 只保留顶层 `Command` 分发
- 把这组命令的 human renderer / JSON error helper 也一起放回主题文件

## 改动概览

- 新增 `rust/src/palace_cli.rs`
  - 提供 `PalaceCommand`
  - 提供 `RepairCommand`
  - 提供 `handle_palace_command()`
  - 收口 `compress/wake-up/recall/layers-status/migrate/repair/dedup/status/doctor/prepare-embedding` 的 CLI wiring
  - 收口这一组命令的人类输出和结构化错误输出
- 更新 `rust/src/main.rs`
  - 引入 `palace_cli`
  - 顶层 `match command` 里的对应分支现在只负责把参数映射到 `PalaceCommand`
  - 删除原来内联在 `main.rs` 里的 palace CLI renderers、repair subcommand enum、doctor/prepare-embedding CLI tests
- 更新 `rust/README.md`
  - 明确把 `palace_cli` 记成 Rust CLI structure 的一部分

## 关键知识

### 为什么这轮把这么多命令一起搬

这批命令虽然功能不同，但在 CLI 结构上有统一主题：

- 都依赖同一类 palace config 解析
- 大多都有相似的 no-palace / human / JSON error 分流
- 都是“palace 本体相关”的子命令，不是 project bootstrap，也不是 registry

因此它们适合作为一个完整命令族一起抽离，而不是零碎地一个一个搬。

### `PalaceCommand` 的作用

`PalaceCommand` 不是取代顶层 `Command`，而是给 `main.rs` 和 `palace_cli.rs` 之间加一层主题边界：

- `main.rs` 负责解析顶层 clap 结构
- `palace_cli.rs` 负责处理 palace 命令族的运行细节

这样做以后，顶层命令入口还保持清楚，但主题内的具体流程和输出逻辑已经不需要再污染主文件。

## 补充知识

这一轮之后，Rust CLI 的拆分已经开始形成模式：

- `registry_cli.rs`
- `palace_cli.rs`

也就是说，CLI 侧现在已经不只是“所有东西都放 main”，而是沿着命令主题逐步形成独立文件。下一轮如果继续收 `main.rs`，就更容易沿着剩余主题再切下一块。

## 验证

本轮执行并通过：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这轮没有改顶层 `Command` enum 的定义位置；它仍然保留在 `main.rs`
- 这轮没有继续拆 bootstrap / mining / search / normalize 那一组 CLI
- 这轮只做结构收口，没有调整 palace-facing 命令的协议语义
