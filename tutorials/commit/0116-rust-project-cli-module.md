# 背景

上一轮已经把 `registry` 和 palace-facing 命令族从 `main.rs` 里拆走了，但顶层入口里仍然还压着另一整块主题内 CLI：

- `init`
- `onboarding`
- `mine`
- `search`
- `split`
- `normalize`

这组命令共同围绕项目 bootstrap、挖掘和 transcript 预处理展开。如果继续把它们和 human/json formatter 一起留在 `main.rs`，主文件仍然会承担太多主题细节。

## 主要目标

- 把 project/bootstrap 命令族从 `main.rs` 收成独立模块
- 让 `main.rs` 进一步退回到顶层 clap 解析和简单分发
- 保持 CLI 外部行为不变，包括：
  - `mine --progress`
  - `search --human`
  - `normalize`
  - `init/onboarding` 的 human/json 错误面

## 改动概览

- 新增 `rust/src/project_cli.rs`
  - 定义 `ProjectCommand`
  - 提供 `handle_project_command()`
  - 收纳 `init/onboarding/mine/search/split/normalize` 的 dispatch
  - 收纳这组命令对应的 human renderers 和 JSON error helpers
- 更新 `rust/src/main.rs`
  - 引入 `mod project_cli;`
  - 顶层 `Command` 在 match 时转换成 `ProjectCommand`
  - 删除原来内联在 `main.rs` 里的 project/bootstrap formatter 与错误 helper
- 更新 `rust/README.md`
  - 把 `project_cli` 记成 Rust CLI structure 的一部分

## 关键知识

### CLI 模块拆分的重点是主题边界

这轮没有改变顶层 `Command` enum 的定义位置，也没有改动 CLI 参数面。真正的收口点是：

- `main.rs` 负责顶层 clap 结构和 very-thin dispatch
- `project_cli.rs` 负责项目命令族自己的运行细节

这样做的好处是：

- `main.rs` 更容易继续瘦身
- 同主题的 formatter、错误文案、进度 wiring 不再散落在主文件底部
- 后续如果要继续拆其它命令族，会更自然

### `resolve_config()` / `create_app()` 这种 helper 适合放在主题模块里

`project_cli` 和 `palace_cli` 都会做这几步：

1. `AppConfig::resolve(...)`
2. 应用 CLI 覆盖项
3. `App::new(...)`
4. 根据 human/json 模式走不同错误面

把这套套路局部收在命令族模块里，可以减少 `main.rs` 里重复出现“resolve + app + human/json error split”的模板代码。

## 补充知识

### Rust 子模块可以复用父模块里的私有 helper

这轮 `project_cli.rs` 继续复用了 `main.rs` 里的：

- `apply_cli_overrides()`
- `palace_exists()`
- `print_no_palace()`

在 Rust 里，子模块可以访问祖先模块中的私有项，所以这种“顶层共享 helper + 主题子模块复用”的结构是成立的。

### 重构 CLI 时，优先搬运行时细节，再搬 clap 形状

这轮没有急着把顶层 `Command` enum 也移出 `main.rs`。先搬：

- 运行逻辑
- human/json formatter
- error helpers

这样风险更小，也更容易验证行为没变。等运行时边界稳定后，再考虑是否有必要继续搬 top-level clap 定义。

## 验证

在 `rust/` 下执行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这轮没有改顶层 `Command` enum 的定义位置；它仍然保留在 `main.rs`
- 这轮没有继续改 `hook`、`instructions`、`mcp` 这些辅助命令的分发结构
- 这轮只收 project/bootstrap CLI 表面，没有新增用户可见功能
