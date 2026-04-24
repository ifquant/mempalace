# 背景

前一轮已经把 Rust 顶层 `palace_cli.rs` 拆成：

- `palace_cli_read`
- `palace_cli_maintenance`
- `palace_cli_embedding`

但 `rust/src/palace_cli_maintenance.rs` 里仍然同时装着三条不同的命令线：

- `migrate`
- `repair`
- `dedup`

而且每条线都自带一套 human/json/no-palace/error renderer。这样继续堆下去，maintenance 这块很快又会长成新的“大文件回流点”。

# 主要目标

把 Rust palace maintenance CLI 再按命令族切开，同时保持外部 surface 不变：

- `palace_cli::handle_palace_command()` 不改调用方式
- `RepairCommand` / `DedupCommand` 继续从原来的 CLI 路径可见
- 用户看到的 CLI 行为、JSON payload、human 文本输出都不变化

# 改动概览

这次新增了四个内部文件：

- `rust/src/palace_cli_migrate.rs`
- `rust/src/palace_cli_repair.rs`
- `rust/src/palace_cli_dedup.rs`
- `rust/src/palace_cli_maintenance_support.rs`

并把 `rust/src/palace_cli_maintenance.rs` 收成一个薄 facade。

## 1. `palace_cli_migrate`

这里现在承接：

- `handle_migrate()`
- migrate 的 human 输出
- migrate 的 JSON error 输出
- no-palace 文本提示

也就是 migration 这条命令线自己的 handler + renderer。

## 2. `palace_cli_repair`

这里现在承接：

- `RepairCommand`
- `handle_repair()`
- `repair`
- `repair scan`
- `repair prune`
- `repair rebuild`

以及对应的：

- human summary
- JSON error
- no-palace 文本

也就是 repair 家族整条命令树的 CLI 面。

## 3. `palace_cli_dedup`

这里现在承接：

- `DedupCommand`
- `handle_dedup()`
- dedup 的 human 输出
- dedup 的 JSON error 输出
- no-palace 文本提示

这样 dedup 不再和 repair/migrate 混在同一个 maintenance 文件里。

## 4. `palace_cli_maintenance_support`

这里现在承接 maintenance 家族共享的 CLI helper：

- `resolve_config()`
- `create_app()`
- `print_json()`

它们本质上是 maintenance 处理器共享的 bootstrap/helper，不属于某一个具体命令。

## 5. `palace_cli_maintenance`

这个文件现在只保留 re-export：

- `RepairCommand`
- `handle_migrate()`
- `handle_repair()`
- `DedupCommand`
- `handle_dedup()`

它的职责变成“maintenance command family 的薄入口”，而不再承载所有实现细节。

# 关键知识

## 1. facade 文件可以只做 re-export

在 Rust 里，一个模块不一定非要自己装实现。对于这种“外部 import 路径需要稳定，但内部实现想继续切开”的场景，一个很实用的做法就是：

- 对外保留原模块名
- 把真正实现放进更细的子模块
- 原模块只做 `pub use`

这样可以同时满足：

- 上层调用方不用跟着改 import
- 内部实现可以继续按职责收口

## 2. handler 和 renderer 通常要一起移动

CLI 模块拆分时，最容易犯的错是：

- handler 在一个文件
- human/json renderer 还留在旧文件

结果最后又形成跨文件来回跳。更稳的切法是按“命令线”切：

- 一个命令的 handler
- 这个命令自己的 human 输出
- 这个命令自己的 JSON error 输出

尽量放在一起。

# 补充知识

## 为什么 shared helper 没继续塞进 `palace_cli_support`

`palace_cli_support` 是 palace CLI 全族共享的 helper；但这次 `resolve_config()` / `create_app()` / `print_json()` 只被 maintenance 家族使用。直接把它们放进 `palace_cli_maintenance_support`，能避免把过于局部的 helper 抬升成“全局共享 API”。

## CLI internal split 的好处不只是文件变短

真正的收益是 review 和后续修改边界更清楚：

- 改 `repair prune` 时，不容易顺手碰到 dedup renderer
- 改 `migrate` 输出时，也不会把 repair subcommand tree 一起拖进 diff

这会让后续“继续大粒度推进”时，每个提交更容易保持单一目标。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

这些检查通过后，可以确认：

- maintenance CLI split 没破坏现有命令 surface
- re-export 结构没有打断编译
- 现有 CLI / MCP / service 回归仍然保持绿色

# 未覆盖项

这次没有继续改：

- `palace_cli_read.rs`
- `palace_cli_embedding.rs`
- `project_cli_*`
- `registry_cli.rs`

因为目标只是把 maintenance CLI 再按命令族切开，而不是继续扩散到所有 CLI 模块。
