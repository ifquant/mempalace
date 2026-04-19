## 背景

前几轮已经把 registry 的核心能力从 `service.rs` 收到 `registry_runtime.rs`，但 CLI 这一层还留着一整块内联逻辑：

- `RegistryCommand` 还定义在 `main.rs`
- registry 子命令的 dispatch 还堆在 `match command` 里
- registry 的 human renderer 也还挂在 `main.rs` 底部

这导致 `main.rs` 继续承担太多与单一命令族相关的细节，不利于后续继续把 CLI 表面按主题拆开。

## 主要目标

- 把 registry 这整块 CLI 表面从 `main.rs` 抽成独立模块
- 让 `main.rs` 只保留顶层命令分发
- 把 registry 的 human renderer 和 command wiring 放回同一个主题文件

## 改动概览

- 新增 `rust/src/registry_cli.rs`
  - 提供 `RegistryCommand`
  - 提供 `handle_registry_command()`
  - 提供 registry 各子命令的人类可读 renderer
- 更新 `rust/src/main.rs`
  - 通过 `mod registry_cli;` 引入模块
  - `Command::Registry { action }` 现在直接委托给 `handle_registry_command()`
  - 删除原来内联在 `main.rs` 里的 registry command dispatch
  - 删除原来内联在 `main.rs` 里的 registry human print helpers
- 更新 `rust/README.md`
  - 明确把 `registry_cli` 记成 Rust CLI structure 的一部分

## 关键知识

### 为什么这轮不把更多命令一起抽出去

这轮刻意只收 registry，而不是顺手把其它 CLI 全拆掉，原因是：

- registry 本身已经有清晰的 runtime 边界：`registry_runtime`
- registry 子命令族内部很完整：`summary / lookup / learn / add_* / query / research / confirm`
- human renderer 也只服务这一组命令

这意味着它是一个天然的“整块抽离”单元，风险低，收益明确。

### CLI 模块拆分的目标不是“功能变化”

这一轮没有改 registry 的行为语义，而是把它的命令 wiring 移到更合适的文件里。这样做的重点是：

- 降低 `main.rs` 的噪音
- 让后续继续拆 `main.rs` 时可以沿着命令主题推进
- 把 command enum、dispatch、human output 放在同一个主题模块里，减少来回跳文件

## 补充知识

这一轮之后，Rust 侧已经有两种比较稳定的收口方向：

- library/runtime 侧：
  - `registry_runtime`
  - `palace_read`
  - `palace_ops`
  - `maintenance_runtime`
  - `init_runtime`
- CLI 侧：
  - `registry_cli`

也就是说，repo 现在不只是把业务逻辑拆出了 `service.rs`，也开始把顶层 CLI wiring 从 `main.rs` 往主题模块收。

## 验证

本轮执行并通过：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这轮只拆了 registry CLI；`main.rs` 里其它命令族还没有继续分模块
- 这轮没有改 registry 的 JSON / human 输出协议，只做结构收口
- 如果后面继续拆 CLI，比较自然的下一批候选会是 layer/maintenance 或 bootstrap 相关命令组
