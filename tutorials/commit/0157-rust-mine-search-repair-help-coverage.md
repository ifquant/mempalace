# 0157 Rust `mine` / `search` / `repair` help 覆盖补齐

## 背景

在连续几轮 help 一致性收口之后，`mine`、`search`、`repair` 这三个高频入口仍有一个共同问题：

- help 本身已经包含较完整的参数语义
- 但测试断言还只锁了其中一小部分标题级文本

这意味着如果后面有人不小心改掉了关键参数说明，测试未必能第一时间发现。

## 主要目标

把 `mine --help`、`search --help`、`repair --help` 的关键参数和子命令语义补进 CLI 集成测试覆盖。

## 改动概览

- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - `cli_mine_help_mentions_human_output`
    - 补锁 `--include-ignored`
    - 补锁 `--no-gitignore`
    - 补锁 `--progress`
  - `cli_search_help_mentions_filters_and_results`
    - 补锁 `<QUERY>` 参数说明 `What to search for`
  - `cli_repair_help_mentions_human_output`
    - 补锁 `scan` 的 `corrupt_ids.txt` 语义
    - 补锁 `prune` 的删除语义
    - 补锁 `rebuild` 的 SQLite -> vector store 重建语义
- 新增本教程文档

## 关键知识

### 1. 参数名和参数语义都要测

只测 `--include-ignored` 这种 flag 名字还不够，因为真正有信息量的是它后面的说明文字。  
不过在这轮里，我们先把“关键参数确实还在 help 里”锁住，再逐步提高覆盖颗粒度。

### 2. `repair` 的 help 特别值得锁

`repair` 不只是一个简单命令名，它下面有明确的子命令语义：

- `scan` 是写 `corrupt_ids.txt`
- `prune` 是按这个文件删 ID
- `rebuild` 是从 SQLite 重建向量库

如果这些 help 描述漂掉，用户会很难从 CLI 层判断各子命令的风险和用途。

### 3. 高频入口优先于冷门入口

在 help 覆盖补齐阶段，最划算的做法不是平均用力，而是优先锁：

- `mine`
- `search`
- `repair`

因为它们是最容易被真实用户直接调用、也是最容易被 README/教程引用的入口。

## 补充知识

### 1. help 覆盖补齐适合按“命令族”连续推进

像这次这样，把三个相关高频命令放在同一个小切片里，比一轮只补一个 help 测试更高效，而且风险仍然很低。

### 2. CLI 测试也可以承担一部分文档回归职责

help 文本本来就是一层文档。  
把它纳入集成测试，等于把一部分文档回归也自动化了。

## 验证

在 `rust/` 目录执行：

```bash
cargo fmt --check
cargo test cli_mine_help_mentions_human_output -- --exact
cargo test cli_search_help_mentions_filters_and_results -- --exact
cargo test cli_repair_help_mentions_human_output -- --exact
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这次没有修改任何 `rust/src/` 实现逻辑
- 这次没有继续扩 `status`、`dedup`、`prepare-embedding` 等其它子命令的 help 断言
- 这次没有修改 README，因为本轮只是在补 help 覆盖，不是在调整示例命令
