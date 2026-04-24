# 背景

前一轮已经把 `main.rs` 里的 project 命令抽到了 `project_cli.rs`，但这个文件很快又开始同时承担三类职责：

- project bootstrap：`init` / `onboarding`
- mining and search：`mine` / `search`
- transcript prep：`split` / `normalize`

这样虽然顶层入口变薄了，但 `project_cli.rs` 自己又重新变成了“新的大文件”。继续往 Python 侧职责边界对齐时，这种结构会让后续改动再次互相缠住。

# 主要目标

把 Rust 的 project-facing CLI 再按工具族切开，让：

- bootstrap 一组单独维护
- mining/search 一组单独维护
- transcript prep 一组单独维护
- `project_cli.rs` 本身退回成薄分发层

目标仍然是**不改行为面，只收紧模块边界**。

# 改动概览

这次新增了四个模块：

- `rust/src/project_cli_bootstrap.rs`
- `rust/src/project_cli_mining.rs`
- `rust/src/project_cli_transcript.rs`
- `rust/src/project_cli_support.rs`

并把原来的 `rust/src/project_cli.rs` 改成只保留：

- `ProjectCommand`
- `handle_project_command()` 顶层路由

具体分工如下：

1. `project_cli_bootstrap`

- 承接 `handle_init()`
- 承接 `handle_onboarding()`
- 承接 init/onboarding 的 human renderer 和 JSON error helper

2. `project_cli_mining`

- 承接 `handle_mine()`
- 承接 `handle_search()`
- 承接 mining/search 的 progress wiring、human renderer、JSON error helper

3. `project_cli_transcript`

- 承接 `handle_split()`
- 承接 `handle_normalize()`
- 承接 normalize 的 preview renderer 和 unsupported transcript 错误输出

4. `project_cli_support`

- 承接 `resolve_config()`
- 承接 `create_app()`
- 承接 `print_json()`

这样 project 命令族的边界就和前面已经拆开的 `palace_cli_*` 更一致了。

# 关键知识

## 1. CLI 模块拆分的核心不是“少代码”，而是“减少跨命令耦合”

`mine` 和 `normalize` 看起来都属于“project 命令”，但它们依赖的运行时完全不同：

- `mine` 依赖 palace config、App bootstrap、embedding/provider 错误分流
- `normalize` 只是单文件 transcript 处理

如果继续放在同一个文件里，后面加任何 `mine` 相关行为时，都更容易误碰 `normalize` 那套输出逻辑。拆开之后，每组命令只对自己的错误面和 renderer 负责。

## 2. 薄 dispatcher 能让顶层 runtime 更稳定

拆完之后：

- `cli_runtime.rs` 仍然只需要认识 `ProjectCommand`
- `project_cli.rs` 只负责把 command 发到正确模块
- 真实实现则沉到 command family 模块

这样以后如果继续拆 `project_cli_mining` 内部的 `mine/search`，就不会再波及顶层路由。

# 补充知识

## 为什么保留 `ProjectCommand` 在 `project_cli.rs`

这次没有把 `ProjectCommand` 也继续拆散，是因为它现在承担的是**命令族公共边界**：

- `cli_runtime.rs` 只需要构造它
- 各个 family 模块只负责具体执行

如果此时把 command enum 继续拆成多个独立类型，反而会让顶层 runtime 需要知道更多模块细节，收益不大。

## `project_cli_support` 和 `cli_support` 的区别

- `cli_support` 是整个 binary 的横向公共 helper
- `project_cli_support` 是 project-facing commands 的纵向公共 helper

前者是全 CLI 通用设施，后者是 project 命令族内部共享设施。这样比把全部 helper 都重新塞回全局 support 文件更清楚。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

这次改动之后，验证通过，说明拆分没有改变现有 project CLI 的行为。

# 未覆盖项

这次没有继续把 `project_cli_mining` 内部再拆成 `mine/search` 两个更细模块，因为当前这一层切分已经足够把：

- bootstrap
- mining/search
- transcript prep

三条主职责断开。

后续如果 `mine/search` 继续膨胀，再单独对 `project_cli_mining` 下刀会更合适。
