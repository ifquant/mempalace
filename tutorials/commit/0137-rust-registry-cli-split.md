# 背景

前面已经把 Rust 的 palace CLI 和 project CLI 按命令族切开，但
`rust/src/registry_cli.rs` 仍然把几类不同节奏的东西堆在同一个文件里：

- clap subcommand schema
- registry read 命令
- registry write 命令
- registry research/confirm 命令
- shared app/bootstrap helper
- human/json renderer

这会让任何一次 registry CLI 小改动都把整块读写研究逻辑一起带进 diff。

# 主要目标

把 Rust registry CLI 再按命令族切开，同时保持外部 surface 不变：

- `RegistryCommand` 继续从 `registry_cli` 暴露
- `handle_registry_command()` 继续作为顶层入口
- `root_cli.rs` 和 `cli_runtime.rs` 不需要跟着改调用方式
- 用户可见的 registry CLI 行为、输出和参数不变化

# 改动概览

这次新增了四个内部文件：

- `rust/src/registry_cli_read.rs`
- `rust/src/registry_cli_write.rs`
- `rust/src/registry_cli_research.rs`
- `rust/src/registry_cli_support.rs`

并把 `rust/src/registry_cli.rs` 收成 clap schema + 薄路由入口。

## 1. `registry_cli_read`

这里现在承接：

- `summary`
- `lookup`
- `learn`
- `query`

以及它们对应的：

- human 输出
- JSON 输出

也就是 registry 的读侧 CLI 面。

## 2. `registry_cli_write`

这里现在承接：

- `add-person`
- `add-project`
- `add-alias`

以及写入成功后的 human/json summary。

这三条命令都属于“显式修改 registry”的写侧命令，放在一个文件里更容易看清边界。

## 3. `registry_cli_research`

这里现在承接：

- `research`
- `confirm`

以及对应的：

- human 输出
- JSON 输出

这两条命令共享 wiki cache / confirm 语义，和 read/write 分开之后，后续要继续扩 `research` 行为时就不会再碰到普通 lookup/write 渲染。

## 4. `registry_cli_support`

这里现在承接 registry CLI 共享 helper：

- `build_registry_app()`
- `print_registry_json()`

也就是 registry 家族自己的 app bootstrap 和通用 JSON 输出，不再和具体 command family 混在一起。

## 5. `registry_cli`

这个文件现在只保留：

- `RegistryCommand`
- `handle_registry_command()`

它的职责变成：

- 提供稳定的 clap schema
- 按 command family 做顶层分发

而不再承载全部实现细节。

# 关键知识

## 1. schema 和 runtime 最好分层

CLI 模块里最容易失控的一个原因是把这些东西全塞进一个文件：

- clap schema
- handler
- renderer
- shared helper

一开始看起来方便，但当命令数变多之后，任何改动都会把“参数定义”和“业务逻辑”一起拖进 review。把 schema 留在 facade，具体执行拆进 family module，会更稳。

## 2. registry 的 research 流程和普通读写节奏不同

`research/confirm` 这条线和 `summary/lookup/query` 的差异很大：

- 它会触发 wiki cache
- 它会涉及 confirm promotion
- 它的输出内容也更长

所以如果继续跟普通读写混在一起，文件会很快重新长回去。单独成 `registry_cli_research`，后续继续演化时更干净。

# 补充知识

## 为什么 `learn` 归到 read family

`learn` 虽然会更新 registry，但从 CLI 使用感上，它更像：

- 扫描项目
- 汇总新发现
- 打印一份“learn summary”

它和 `summary/lookup/query` 共享更多“读结果/展示结果”的风格，而不是 `add-person` 这种显式单条写操作。所以这次把它放进 `registry_cli_read` 更贴合当前命令族边界。

## `unreachable!()` 在这种薄分发里是合理的

拆成 family handler 之后，每个 handler 只应该接到自己负责的命令分支。这里用 `unreachable!()` 表达的是：

- 顶层 router 已经保证分发正确
- 如果还能落到错误 family，说明是代码组织出了 bug

对这种内部不变量来说，这比静默吞掉分支更好。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

这些检查通过后，可以确认：

- registry CLI split 没破坏现有 subcommand surface
- 新的 facade + family handler 结构没有打断编译
- 现有 CLI / service / MCP 回归仍然保持绿色

# 未覆盖项

这次没有继续改：

- `normalize.rs`
- `embed.rs`
- `hook.rs`
- `mcp_runtime_registry.rs`

因为目标只是把 registry CLI 再按命令族切开，而不是继续扩散到库层 registry runtime 或 MCP runtime。
