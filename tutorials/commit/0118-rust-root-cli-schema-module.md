# 背景

前几轮已经把 Rust CLI 按主题拆出了：

- `registry_cli`
- `palace_cli`
- `project_cli`
- `helper_cli`
- `cli_support`

但 `main.rs` 里还保留着最后一大块“顶层 clap 形状”：

- `Cli`
- `Command`
- 所有顶层 subcommand 的参数定义

这意味着主入口虽然已经不再负责业务逻辑，但仍然还在承担完整的 CLI schema 定义。

## 主要目标

- 把顶层 clap schema 从 `main.rs` 收成独立模块
- 让 `main.rs` 真正只剩：
  - parse
  - route
- 不改变任何用户可见参数面和命令行为

## 改动概览

- 新增 `rust/src/root_cli.rs`
  - 定义顶层 `Cli`
  - 定义顶层 `Command`
  - 复用已有模块里的：
    - `RepairCommand`
    - `HookCommand`
    - `RegistryCommand`
- 更新 `rust/src/main.rs`
  - 改为从 `root_cli` 引入 `Cli` / `Command`
  - 删除原来内联在 `main.rs` 里的顶层 clap 结构
  - 顺手清理因此产生的 unused imports
- 更新 `rust/README.md`
  - 把 `root_cli` 记成 Rust CLI structure 的一部分

## 关键知识

### CLI schema 模块和 CLI handler 模块不是一回事

这轮新增的 `root_cli.rs` 不负责执行命令，它只负责“命令长什么样”。  
也就是说，现在 CLI 侧大致分成三层：

- `root_cli`
  - 顶层参数和 subcommand 结构
- `*_cli`
  - 各主题命令族的 handler
- `main.rs`
  - parse 后把命令路由到对应 handler

这个边界和前面拆出来的 runtime / service / library 模块边界是一致的：  
schema、dispatch、implementation 各自分开。

### `main.rs` 的理想形态通常不是“零逻辑”，而是“只剩路由逻辑”

这轮之后，`main.rs` 并没有变成空壳文件，它仍然负责：

- `Cli::parse()`
- 顶层 `match command`
- 把顶层命令转成 `ProjectCommand` / `PalaceCommand` / `HelperCommand`

这通常就是一个 CLI 程序入口比较健康的形态：  
它可以保留路由，但不再拥有具体参数 schema 和命令实现细节。

## 补充知识

### 先拆 handler，再拆 schema，能降低重构风险

如果在一开始 `main.rs` 还很胖的时候就把 `Cli/Command` 抽走，  
经常会出现：

- 一个 schema 模块仍然依赖大量主文件内部 helper
- handler 和 schema 边界不清

这轮是在前面已经把主题 handler 基本收干净后，才继续抽顶层 schema，  
所以落地会更顺。

### 顶层 enum 外移后，unused import 很容易成片出现

这轮一个很典型的小收尾就是：

- `PathBuf`
- `HookCommand`
- `RepairCommand`
- `RegistryCommand`

在 `main.rs` 里都不再直接被用到。  
这种 warning 不是功能问题，但如果仓库要求 `clippy -D warnings`，就必须顺手收干净。

## 验证

在 `rust/` 下执行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这轮没有改变任何 CLI flag、默认值或帮助文本语义
- 这轮没有继续搬顶层 route match；它仍然保留在 `main.rs`
- 这轮是结构收口，不新增功能
