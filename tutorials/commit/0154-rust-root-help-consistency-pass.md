# 0154 Rust 顶层 help 一致性收口

## 背景

上一轮已经把 Rust/Python 的 CLI 和 MCP 表面做成了 parity ledger，但顶层 `mempalace-rs --help` 的长描述还停留在更早阶段：

- 只提到了 mining/search/compress/wake-up/migrate/repair/MCP
- 没有反映已经存在的 `onboarding`、`normalize`、`recall`、`registry` 等表面

这样会出现一个问题：代码和总账已经更新了，但用户第一眼看到的入口帮助还在描述旧状态。

## 主要目标

把 Rust 顶层 CLI help、README 顶部 parity 摘要、以及 root help 集成测试断言同步到同一个现实版本。

## 改动概览

- 更新 [rust/src/root_cli.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/root_cli.rs)
  - 改写顶层 `long_about`
  - 让它覆盖当前更完整的表面：bootstrap、transcript prep、recall、registry、maintenance、MCP
  - 刷新示例命令
- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - 扩充 root help 的断言
  - 不再只盯旧的核心命令
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 在顶部 parity 摘要里补一条，明确顶层 help 也已经反映当前更大的 CLI 面

## 关键知识

### 1. `--help` 也是用户可见 API

很多时候大家只把命令和 flag 当 API，但实际用户最先接触到的是 help 文本。  
如果 help 停在旧阶段，用户会直接低估系统能力，或者误以为某些命令还不存在。

### 2. parity ledger 不能和 help 文本脱节

上一轮 ledger 已经写明 Rust CLI 是 Python CLI 的超集。  
如果顶层 help 还像旧版本那样只列一小部分能力，就会形成三层不同步：

- 代码真实能力
- parity ledger
- CLI help

这次就是把这三层收回到同一口径。

### 3. 测试断言要盯“当前入口现实”，不是历史印象

`cli_root_help_mentions_core_commands_and_examples()` 这种测试，如果长期只断言一组旧命令，会让帮助文本的演进被无意中压住。  
更好的做法是让它覆盖当前真正重要的入口族。

## 补充知识

### 1. 文案一致性是一种低风险高收益收口

这类改动不碰持久化、不碰核心算法，但能立刻减少误导。  
在重写后期，这通常是很值得优先做的一类收口。

### 2. “示例命令” 比“命令列表”更能暴露文案是否过时

命令列表来自 clap schema，通常不太会漏。  
真正容易过时的是 `long_about` 和 examples，因为它们是手写的、最容易停留在旧阶段。

## 验证

在 `rust/` 目录执行：

```bash
cargo fmt --check
cargo test cli_root_help_mentions_core_commands_and_examples -- --exact
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

并额外人工查看：

```bash
cargo run -- --help
```

确认顶层帮助已反映当前 CLI 面。

## 未覆盖项

- 这次没有修改任何 `python/` 实现或文档
- 这次没有继续关闭 parity ledger 里的其它 `remaining` 项
- 这次只收了顶层 root help，没有逐个子命令做完整 help 文案审计
