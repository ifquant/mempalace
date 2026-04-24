# 背景

前几轮已经把 Rust 的 palace CLI 拆成：

- read family
- maintenance family
- embedding family

其中 `rust/src/palace_cli_embedding.rs` 虽然只负责两条命令：

- `doctor`
- `prepare-embedding`

但它还是同时装着：

- handler
- human/json error renderer
- 文本渲染 helper
- shared config/app bootstrap
- 单元测试

继续这样放着，embedding 这条线也会慢慢重新长回“大文件”。

# 主要目标

把 Rust embedding CLI 再按命令族切开，同时保持外部 surface 不变：

- `handle_doctor()` 继续从 `palace_cli_embedding` 暴露
- `handle_prepare_embedding()` 继续从 `palace_cli_embedding` 暴露
- `palace_cli.rs` 和上层 palace command dispatch 不需要改调用方式
- 用户可见的输出和错误行为不变化

# 改动概览

这次新增了三个内部文件：

- `rust/src/palace_cli_embedding_doctor.rs`
- `rust/src/palace_cli_embedding_prepare.rs`
- `rust/src/palace_cli_embedding_support.rs`

并把 `rust/src/palace_cli_embedding.rs` 收成一个薄 facade。

## 1. `palace_cli_embedding_doctor`

这里现在承接：

- `handle_doctor()`
- doctor 的 human 渲染
- doctor 的 JSON error 输出
- doctor 的单元测试

也就是 `doctor` 这条命令线的完整 CLI 面。

## 2. `palace_cli_embedding_prepare`

这里现在承接：

- `handle_prepare_embedding()`
- prepare-embedding 的 human 渲染
- prepare-embedding 的 JSON error 输出
- prepare-embedding 的单元测试

这条命令和 doctor 共享 embedding 语义，但它自己的输出结构和提示文案已经足够独立，单独放出去更清楚。

## 3. `palace_cli_embedding_support`

这里现在承接 embedding family 共享 helper：

- `resolve_embedding_config()`
- `create_embedding_app()`
- `print_embedding_json()`

这样 shared bootstrap 不再混在 doctor/prepare 任何一条具体命令线里。

## 4. `palace_cli_embedding`

这个文件现在只保留 re-export：

- `handle_doctor()`
- `handle_prepare_embedding()`

它的职责变成“embedding command family 的薄入口”，不再承载具体实现。

# 关键知识

## 1. renderer 和命令逻辑最好一起移动

CLI internal split 时，如果只把 handler 拆出去，但把 render helper 还留在原文件，最后还是要跨文件来回跳。更稳的切法是：

- doctor handler
- doctor human renderer
- doctor JSON error renderer
- doctor test

放在一个文件里。prepare-embedding 也是同理。

## 2. shared helper 不一定都该进通用 support

虽然仓库里已经有 `palace_cli_support`，但这次的 config/app/json helper 只服务 embedding family。继续往“更全局”的 support 抬，会把局部 helper 误提升成全局公共 API。单独放在 `palace_cli_embedding_support`，边界更清楚。

# 补充知识

## 为什么把 doctor 测试一起移走

`doctor_human_failure_suggests_mirror_when_default_endpoint_fails` 这种测试，本质上是在验证 doctor renderer 的文本 contract。把它留在旧 facade 文件里，会让测试和真正实现分离。跟实现一起搬走，后续改 renderer 时更容易保持测试同步。

## facade re-export 很适合这种渐进拆分

这类 Rust 模块收口最实用的方式之一就是：

- 真实实现继续按 family 拆开
- 原模块只做 `pub use`

这样既不会打断上层 import，也不会让新的内部拆分影响对外 surface。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

这些检查通过后，可以确认：

- embedding CLI split 没破坏现有命令 surface
- 新的 facade + family helper 结构没有打断编译
- 现有 CLI / service / MCP 回归仍然保持绿色

# 未覆盖项

这次没有继续改：

- `normalize.rs`
- `embed.rs`
- `hook.rs`
- `mcp_runtime_read.rs`

因为目标只是把 embedding CLI 再按命令族切开，而不是继续扩散到 embedding runtime 或更底层库模块。
