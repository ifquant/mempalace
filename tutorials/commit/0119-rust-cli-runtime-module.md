# 背景

上一轮已经把顶层 clap schema 从 `main.rs` 抽成了 `root_cli.rs`，所以主文件里只剩：

- `Cli::parse()`
- 一个顶层 `match command`
- 把每个顶层命令转给对应的 `*_cli` 模块

这已经比最早干净很多了，但严格来说，顶层 route 仍然还绑在 binary entrypoint 里。  
如果继续沿着“schema / route / handler 分层”推进，那么下一步自然就是把 route 本身也独立出来。

## 主要目标

- 把顶层 route 从 `main.rs` 抽成独立 `cli_runtime` 模块
- 让 `main.rs` 真正只剩：
  - `Cli::parse()`
  - `run_cli(...)`
- 保持全部 CLI 表面和行为不变

## 改动概览

- 新增 `rust/src/cli_runtime.rs`
  - 提供 `run_cli(cli: Cli)`
  - 接管原来写在 `main.rs` 里的顶层 `match command`
  - 继续把顶层命令路由到：
    - `project_cli`
    - `palace_cli`
    - `helper_cli`
    - `registry_cli`
- 更新 `rust/src/main.rs`
  - 引入 `cli_runtime`
  - 删除原来内联的顶层 route
  - 现在只保留 `Cli::parse()` + `run_cli(...)`
  - 顺手清理 route 外移后产生的 unused import
- 更新 `rust/README.md`
  - 把 `cli_runtime` 记成 Rust CLI structure 的一部分

## 关键知识

### `root_cli` 和 `cli_runtime` 分别解决不同问题

它们虽然都属于“顶层 CLI”，但边界不同：

- `root_cli`
  - 负责命令长什么样
  - 即：schema / flags / subcommands
- `cli_runtime`
  - 负责命令怎么被分发
  - 即：route / dispatch

拆开后，binary 侧就形成了清晰三层：

- `root_cli`: 定义
- `cli_runtime`: 路由
- `*_cli`: 各主题命令族实现

### `main.rs` 最后应该更像入口桩，而不是控制中心

这轮之后，`main.rs` 基本只做两件事：

1. `Cli::parse()`
2. `run_cli(...)`

这意味着真正的控制流已经不再由入口文件持有。  
对后续维护的价值是：

- 更容易测试和阅读 dispatch 逻辑
- 更容易继续演化 binary 层而不反复修改入口文件
- 顶层文件变得非常稳定

## 补充知识

### 当入口已经很薄时，再拆 route 的收益会变大

如果主文件还很胖，先拆 route 往往收益一般，因为 route 周围还会缠着一堆 formatter、helper、schema。  
但在前几轮已经把：

- helper
- schema
- 各主题 handler

都拆掉之后，再抽 route，入口就能真正收成一个稳定外壳。

### 清理 unused import 是这类结构重构的最后一步

这轮真正出现的唯一编译问题不是逻辑错误，而是：

- `Command` 在 `main.rs` 里已经不再直接用到

这类 warning 很小，但在 `clippy -D warnings` 的仓库里一样会阻塞提交，所以结构重构后最好先做一轮 `cargo check` 快速找掉这些尾巴。

## 验证

在 `rust/` 下执行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这轮没有改变任何 CLI 参数、帮助文本或输出协议
- 这轮没有继续把 binary-only 模块迁成 library API
- 这轮只是把顶层 route 收口，不新增功能
