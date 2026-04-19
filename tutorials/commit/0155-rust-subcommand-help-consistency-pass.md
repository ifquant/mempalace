# 0155 Rust 子命令 help 与 README 示例继续对齐

## 背景

上一轮已经把顶层 `mempalace-rs --help` 收到了当前真实 surface，但还有一层更细的残项：

- `onboarding --help` 实际已经支持 `--auto-accept-detected` 和 `--human`，测试却没锁住
- `mcp --help` 实际已经解释了 `--serve` 的 stdio server 语义，测试覆盖还不完整
- README 的验证命令列表里，也还没把 `recall`、`layers-status`、`mcp --serve` 这些已公开表面补进去

这类差异不会让代码坏掉，但会让文档、help、测试继续慢慢漂开。

## 主要目标

继续收 docs/help/test 一致性，把这三个表面的描述同步到同一状态。

## 改动概览

- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - 扩充 `onboarding --help` 断言
  - 扩充 `mcp --help` 断言
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 在 “Useful verification command” 里加入：
    - `recall`
    - `layers-status`
    - `mcp --serve`
- 新增本教程文档

## 关键知识

### 1. 子命令 help 也需要测试锁定

顶层 `--help` 收口后，如果子命令 help 不继续跟上，漂移还是会发生。  
像 `onboarding` 这种参数比较多的命令，help 文案本身就很容易成为“用户真实接口”。

### 2. README 示例命令应该覆盖真实常用入口

验证命令列表不是越长越好，但至少应该覆盖当前已经公开、而且用户真的会试的入口族。  
`recall`、`layers-status`、`mcp --serve` 都属于这一类。

### 3. 最容易过时的是“辅助说明”，不是实现本身

实现代码一般不会悄悄丢功能，但测试断言、README 示例和 help 描述如果不跟着更，就很容易卡在旧阶段。  
所以这类同步收口适合持续做小切片。

## 补充知识

### 1. 一致性收口不一定要改代码逻辑

很多高价值提交并不改变行为，而是把行为的描述、入口、验证方式重新锁紧。  
这种提交风险低，但对后续继续推进很重要。

### 2. “示例命令” 是另一种接口文档

用户往往不会先读整份 README，而是直接抄示例命令。  
所以示例里漏掉某个公开命令，实际效果就和“这个命令不重要”差不多。

## 验证

在 `rust/` 目录执行：

```bash
cargo fmt --check
cargo test cli_onboarding_help_mentions_mode_people_and_scan -- --exact
cargo test cli_mcp_help_mentions_setup_and_serve_flags -- --exact
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

并额外人工查看：

```bash
cargo run -- onboarding --help
cargo run -- mcp --help
```

## 未覆盖项

- 这次没有改任何 `rust/src/` 实现逻辑
- 这次没有继续审其它子命令 help，例如 `repair`、`registry`、`normalize` 的更细文案
- 这次没有修改 `docs/parity-ledger.md`，因为 ledger 的结论本身没有变化
