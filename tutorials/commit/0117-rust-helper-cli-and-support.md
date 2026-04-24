# 背景

在前两轮里，`main.rs` 已经先后把：

- `registry` 命令族
- palace-facing 命令族
- project/bootstrap 命令族

拆成了独立模块，但主文件里还剩一块明显成组的 control-plane 表面：

- `hook`
- `instructions`
- `mcp`

此外，一些跨模块共用的 CLI helper 也还挂在 `main.rs` 根部：

- `apply_cli_overrides()`
- `palace_exists()`
- `print_no_palace()`
- `print_mcp_setup()`
- `shell_quote()`

这会让 `main.rs` 虽然变薄了，但仍然同时承担“辅助命令族实现”和“共享 CLI 工具箱”两种责任。

## 主要目标

- 把 `hook/instructions/mcp` 从 `main.rs` 收成独立 `helper_cli` 模块
- 把跨 CLI 模块的共享 helper 收成独立 `cli_support` 模块
- 继续让 `main.rs` 退回到更纯粹的顶层 clap 解析和分发
- 不改变用户可见行为

## 改动概览

- 新增 `rust/src/helper_cli.rs`
  - 定义 `HelperCommand`
  - 定义 `HookCommand`
  - 提供 `handle_helper_command()`
  - 收纳 `hook`、`instructions`、`mcp` 三组辅助命令的分发和运行
- 新增 `rust/src/cli_support.rs`
  - 提供 `apply_cli_overrides()`
  - 提供 `palace_exists()`
  - 提供 `print_no_palace()`
  - 提供 `shell_quote()`
  - 提供 `format_mcp_setup()`
- 更新 `rust/src/main.rs`
  - 引入 `helper_cli` / `cli_support`
  - 删除原来内联在 `main.rs` 里的 helper command dispatch
  - 删除原来挂在 `main.rs` 底部的共享 CLI helper
- 更新 `rust/src/project_cli.rs`
- 更新 `rust/src/palace_cli.rs`
- 更新 `rust/src/registry_cli.rs`
  - 统一改为从 `cli_support` 复用 shared helpers
- 更新 `rust/README.md`
  - 把 `helper_cli` 和 `cli_support` 记成 Rust CLI structure 的一部分

## 关键知识

### “命令族模块”和“共享 helper 模块”是两种不同拆法

这轮不是只新增一个 `helper_cli.rs` 就结束，而是把剩下的 CLI 代码拆成两层：

- `helper_cli`
  - 面向一组相关命令
  - 负责 dispatch 和命令执行
- `cli_support`
  - 面向多个模块复用
  - 负责通用 helper

如果把所有 helper 都塞进 `helper_cli`，那 `project_cli`、`palace_cli`、`registry_cli` 仍然会依赖 `main.rs` 或重新复制逻辑。单独做一层 `cli_support`，边界会更干净。

### `format_mcp_setup()` 比直接打印更容易复用

原先 `print_mcp_setup()` 直接在 `main.rs` 里输出。现在改成：

- `format_mcp_setup()` 负责拼装完整文本
- `helper_cli` 决定何时打印

这样的好处是：

- 更容易在不同调用点复用
- 更容易测试字符串输出
- 模块边界更清晰：格式化和输出动作不再绑死在一起

## 补充知识

### 先抽主题，再抽共享底座，是低风险重构顺序

前几轮先把 `registry_cli`、`palace_cli`、`project_cli` 抽出来，再在这一轮统一抽 `cli_support`，风险更低。  
如果一开始就先抽所有共享 helper，很容易在主文件仍然很乱的时候把依赖关系越抽越复杂。

### `main.rs` 变薄后，剩余职责会更容易暴露

这轮之后，`main.rs` 主要只剩：

- 顶层 `Cli` / `Command` clap 结构
- 把顶层参数映射到各主题模块

一旦主文件接近这个形态，下一轮要继续重构时，就更容易判断：

- 是不是还需要继续搬 top-level enum
- 还是已经达到了“顶层入口只做路由”的合适平衡点

## 验证

在 `rust/` 下执行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这轮没有移动顶层 `Command` enum 的定义位置；它仍然保留在 `main.rs`
- 这轮没有改用户可见 CLI flag 或输出协议
- 这轮只是把辅助命令和共享 helper 收口，没有新增功能
