# 背景

前面已经把 Rust 的 project CLI 拆成三块：

- `project_cli_bootstrap`
- `project_cli_mining`
- `project_cli_transcript`

但 `rust/src/project_cli_bootstrap.rs` 里仍然同时装着两条不同节奏的命令线：

- `init`
- `onboarding`

并且还混着：

- handler
- human/json renderer
- onboarding 参数解析循环
- shared bootstrap helper

如果继续这样堆着，bootstrap family 很快也会重新长成新的大文件。

# 主要目标

把 Rust project bootstrap CLI 再按命令族切开，同时保持外部 surface 不变：

- `handle_init()` 继续从 `project_cli_bootstrap` 暴露
- `handle_onboarding()` 继续从 `project_cli_bootstrap` 暴露
- `project_cli.rs` 和顶层 CLI dispatch 不需要改调用方式
- 用户可见的 init/onboarding 输出和错误行为不变化

# 改动概览

这次新增了三个内部文件：

- `rust/src/project_cli_bootstrap_init.rs`
- `rust/src/project_cli_bootstrap_onboarding.rs`
- `rust/src/project_cli_bootstrap_support.rs`

并把 `rust/src/project_cli_bootstrap.rs` 收成一个薄 facade。

## 1. `project_cli_bootstrap_init`

这里现在承接：

- `handle_init()`
- init 的 human 输出
- init 的 JSON error 输出

也就是 `init` 这条命令线自己的 CLI 面。

## 2. `project_cli_bootstrap_onboarding`

这里现在承接：

- `handle_onboarding()`
- onboarding 的 people/alias 参数解析
- onboarding 的 human 输出
- onboarding 的 JSON error 输出

这条命令和 init 同属 bootstrap family，但它自己的参数解析和展示逻辑明显更复杂，拆出去之后边界更清楚。

## 3. `project_cli_bootstrap_support`

这里现在承接 bootstrap family 共享 helper：

- `resolve_bootstrap_config()`
- `create_bootstrap_app()`
- `print_bootstrap_json()`

这些 helper 只服务 bootstrap family，不必继续和其它 project CLI 共享层混在一起。

## 4. `project_cli_bootstrap`

这个文件现在只保留 re-export：

- `handle_init()`
- `handle_onboarding()`

它的职责变成“bootstrap command family 的薄入口”，而不再承载具体实现。

# 关键知识

## 1. bootstrap family 里的 `init` 和 `onboarding` 节奏不同

虽然它们都属于 project bootstrap，但维护节奏不一样：

- `init` 更偏 palace/storage/bootstrap 文件落盘
- `onboarding` 更偏参数收集、world bootstrap、entity/alias 组织

把它们混在一个文件里，会让任何一侧的小改动都把另一侧的渲染和参数逻辑一起拖进 diff。

## 2. 参数解析循环应该跟命令线走

`onboarding` 里最容易长出来的部分，不是调用 `run_onboarding()` 本身，而是：

- `people`
- `aliases`
- `wings`
- 参数错误输出

这类逻辑本来就是 onboarding command contract 的一部分，所以这次把它和 handler、renderer 放在一个文件里，后续改 onboarding CLI 时更容易一处看全。

# 补充知识

## 为什么 `print_bootstrap_json()` 单独抽出来

这轮如果继续直接复用 `project_cli_support::print_json()` 也能工作，但 bootstrap family 已经在走自己的 helper 层。把 JSON 输出统一留在 `project_cli_bootstrap_support`，能让：

- `init`
- `onboarding`

在内部保持一套稳定的 support surface，而不是继续跨层依赖更外侧的 helper。

## facade re-export 适合这种渐进拆分

这种内部切法的目标不是改上层 import，而是让内部结构继续细化。所以最稳的做法仍然是：

- 真正实现继续拆到更细文件
- 原 family 模块只做 `pub use`

这样可以继续让上层保持稳定，而不会被内部模块名变化影响。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

这些检查通过后，可以确认：

- bootstrap CLI split 没破坏现有命令 surface
- 新的 facade + support 结构没有打断编译
- 现有 CLI / service / MCP 回归仍然保持绿色

# 未覆盖项

这次没有继续改：

- `project_cli_mining.rs`
- `project_cli_transcript.rs`
- `normalize.rs`
- `onboarding.rs`

因为目标只是把 project bootstrap CLI 再按命令族切开，而不是继续扩散到 mining/transcript family 或库层 onboarding 模块。
