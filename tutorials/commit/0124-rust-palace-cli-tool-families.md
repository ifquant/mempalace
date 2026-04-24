# 背景

随着前几轮把 `main.rs` 变薄，Rust CLI 的大头已经转移到了 `palace_cli.rs`。它负责 palace-facing 命令族的分发、配置解析、错误出口、人类输出 renderer 和 JSON error payload，最后长到了上千行。

这类文件的典型问题不是“代码写不下”，而是后续加命令时很难快速判断：这个逻辑属于哪一组、应该放哪段、会不会顺手改坏别的命令。

# 主要目标

这次的目标是把 `palace_cli.rs` 也按命令族切开：

- read-side 命令单独一层
- maintenance 命令单独一层
- embedding/runtime 命令单独一层
- 共用 CLI helper 单独一层

同时让顶层 `palace_cli.rs` 只保留命令定义和分发。

# 改动概览

- 新增 `rust/src/palace_cli_read.rs`
- 新增 `rust/src/palace_cli_maintenance.rs`
- 新增 `rust/src/palace_cli_embedding.rs`
- 新增 `rust/src/palace_cli_support.rs`
- `rust/src/palace_cli.rs` 现在只负责：
  - `PalaceCommand`
  - `RepairCommand` re-export
  - `handle_palace_command()` 顶层 dispatch
- `rust/src/main.rs` 增加新的 palace CLI family 模块声明
- `rust/README.md` 同步说明新的 palace CLI family 分层

# 关键知识

这次和前面 MCP 的拆法是同一个原则：让“命令分组”和“文件分组”一致。

现在 palace-facing CLI 大致分成三类：

- read-side: `compress`、`wake-up`、`recall`、`layers-status`、`status`
- maintenance: `migrate`、`repair`、`dedup`
- embedding/runtime: `doctor`、`prepare-embedding`

这样拆之后，如果有人要改 `repair` 的 human output，不需要再在同一个文件里跨过 `status`、`wake-up`、`doctor` 的 renderer 才能定位目标。

另一个关键点是这次没有改变任何命令外部语义。参数、错误路径、人类输出文本和 JSON payload 都保持原样，只是把它们移到了更清晰的文件边界。

# 补充知识

1. CLI 重构时，“把 renderer 和 handler 放在一起”通常比“把所有 renderer 单独集中”更容易维护。因为命令行为和输出格式本来就是一对自然耦合。

2. 顶层 dispatcher 最好保持非常薄。这样以后再继续拆分时，只需要移动某一组 handler，不需要反复改动命令入口结构。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有改动 CLI 参数或外部输出语义
- 这次没有继续拆 `project_cli.rs` 或 `registry_cli.rs`
- 这次没有改动 `python/`、`hooks/`、`docs/`、`assets/`、`.github/` 或其他子树
