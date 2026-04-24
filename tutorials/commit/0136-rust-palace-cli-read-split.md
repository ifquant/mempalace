# 背景

上一轮已经把 Rust 的 palace maintenance CLI 按命令族切开：

- `palace_cli_migrate`
- `palace_cli_repair`
- `palace_cli_dedup`

但与它平行的 `rust/src/palace_cli_read.rs` 仍然同时装着五条读侧命令线：

- `compress`
- `wake-up`
- `recall`
- `layers-status`
- `status`

而且每条线都带着自己的 human/json/no-palace/error renderer。继续堆在一起的话，read 家族也会重新长成一个新的大文件。

# 主要目标

把 Rust palace read CLI 再按命令族切开，同时保持外部 surface 不变：

- `palace_cli::handle_palace_command()` 不改调用路径
- `handle_compress()` / `handle_wake_up()` / `handle_recall()` / `handle_layers_status()` / `handle_status()` 继续从 `palace_cli_read` 暴露
- CLI 的 human 输出、JSON payload、错误行为都不变化

# 改动概览

这次新增了四个内部文件：

- `rust/src/palace_cli_read_compress.rs`
- `rust/src/palace_cli_read_layers.rs`
- `rust/src/palace_cli_read_status.rs`
- `rust/src/palace_cli_read_support.rs`

并把 `rust/src/palace_cli_read.rs` 收成一个薄 facade。

## 1. `palace_cli_read_compress`

这里现在承接：

- `handle_compress()`
- compress 的 human 输出
- compress 的 JSON error 输出
- compress 的 no-palace 文本

也就是 compression 这条读侧命令线自己的 handler + renderer。

## 2. `palace_cli_read_layers`

这里现在承接：

- `handle_wake_up()`
- `handle_recall()`
- `handle_layers_status()`

以及这三条命令自己的：

- human 输出
- JSON error 输出
- no-palace 文本

这三条命令都属于 layer/readback 语义，放在一起比继续塞进同一个大 read 文件更自然。

## 3. `palace_cli_read_status`

这里现在承接：

- `handle_status()`
- `status` 的 human 输出
- taxonomy 排序后的展示逻辑
- `status` 的 JSON error 输出

`status` 这条线有额外的 taxonomy 拉取和人类可读渲染，所以单独分出去更稳。

## 4. `palace_cli_read_support`

这里现在承接 read 家族共享的 helper：

- `resolve_read_config()`
- `create_read_app()`
- `exit_if_no_palace_human_or_json()`
- `print_read_json()`

这些 helper 只服务 read 家族，不必继续挤进全局 `palace_cli_support`。

## 5. `palace_cli_read`

这个文件现在只保留 re-export：

- `handle_compress()`
- `handle_wake_up()`
- `handle_recall()`
- `handle_layers_status()`
- `handle_status()`

它的职责变成“read command family 的薄入口”，而不再承载全部实现。

# 关键知识

## 1. facade 的价值是稳定上层 import

这种二次拆分里，最重要的不是“文件变短”本身，而是：

- 上层 import 路径不变
- 内部实现可以继续切开

也就是：

- 上层继续写 `use crate::palace_cli_read::handle_status;`
- 真实实现已经挪进 `palace_cli_read_status`

这样既保住外部稳定性，也给后续内部继续收口留出空间。

## 2. command family 比 function-by-function 更适合 CLI 拆分

CLI 模块最容易出现的坏味道是“一个 handler 一个文件”，最后到处都是零碎文件，反而更难找。更稳的方式是按命令族拆：

- compress 一族
- layers/readback 一族
- status 一族

这样每个文件都还能保持一个可读的局部上下文。

# 补充知识

## 为什么 `wake-up`、`recall`、`layers-status` 放一起

这三条命令都属于 Python `layers.py` 那条 read-side 叙事链：

- `wake-up` 更偏 Layer 0 / Layer 1
- `recall` 更偏 Layer 2
- `layers-status` 是对整条 layer stack 的摘要

它们虽然是三个命令，但语义上是同一个 layer family，所以这次一起收进 `palace_cli_read_layers` 更顺。

## `status` 单独分出去的原因

`status` 不只是“再打印一个 summary”，它还有：

- `taxonomy()` 的额外读取
- wing/room 计数排序
- 空 palace / 空 drawer 的专门输出

这类命令如果继续跟其它 read 命令混在一起，最容易把 read file 又拖回“大杂烩”。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

这些检查通过后，可以确认：

- read CLI split 没破坏现有命令 surface
- 新 facade/re-export 结构没有打断编译
- 现有 CLI / MCP / service 回归仍然保持绿色

# 未覆盖项

这次没有继续改：

- `palace_cli_embedding.rs`
- `project_cli_*`
- `registry_cli.rs`
- `normalize.rs`

因为目标只是把 palace read CLI 再按命令族切开，而不是继续扩散到其它 CLI 家族或库层模块。
