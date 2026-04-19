# 0158 Rust `status` / `dedup` / `prepare-embedding` help 覆盖补齐

## 背景

help 一致性这条线已经连续收了几轮，但还有三类高频入口的帮助文本没有被测试锁得足够细：

- `status --help`
- `dedup --help`
- `prepare-embedding --help`

它们都已经输出了更完整的参数说明，但测试还主要停留在标题级覆盖。

## 主要目标

把这三个入口的关键 help 语义补进 CLI 集成测试，继续缩小 help 文案和测试覆盖之间的空档。

## 改动概览

- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - `cli_status_help_mentions_human_output`
    - 补锁 usage 行
  - `cli_dedup_help_mentions_threshold_and_stats`
    - 补锁 `--dry-run`
    - 补锁 `--wing`
    - 补锁 `--source`
  - `cli_prepare_embedding_help_mentions_human_output`
    - 补锁 `--attempts`
    - 补锁 `--wait-ms`
- 新增本教程文档

## 关键知识

### 1. help 覆盖要逐步从标题走向参数语义

标题级断言只能说明“这个命令还存在”，但不能说明用户真正依赖的参数说明还在。  
像 `prepare-embedding` 这种命令，真正重要的是重试次数和等待时间这些语义。

### 2. `dedup` 的风险语义值得特别锁定

`dedup` 这类命令天然带风险，因为它会删内容。  
所以它的 help 里：

- `Preview without deleting`
- `Scope dedup to one wing`
- `Filter by source file pattern`

这些都是非常值得测试锁住的关键信号。

### 3. `status` 的 usage 也有价值

`status` 看起来很简单，但它是最常被用户拿来确认“系统到底有没有东西”的入口。  
把 usage 行一起锁住，可以避免后续文案演进时把最基础的入口说明弄漂。

## 补充知识

### 1. 高风险命令的 help 往往比普通命令更值得测

像 `dedup`、`repair` 这种命令，help 文案本身就在承担“安全提示”的作用。  
对这类命令，帮助文本不是附属品，而是接口的一部分。

### 2. 连续小切片能把 help/test 一致性做得很扎实

一次性把所有 help 都补满当然也行，但连续做小切片更容易保持：

- 范围小
- 回归清楚
- 提交语义明确

这对后期收口尤其合适。

## 验证

在 `rust/` 目录执行：

```bash
cargo fmt --check
cargo test cli_status_help_mentions_human_output -- --exact
cargo test cli_dedup_help_mentions_threshold_and_stats -- --exact
cargo test cli_prepare_embedding_help_mentions_human_output -- --exact
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这次没有修改任何 `rust/src/` 实现逻辑
- 这次没有继续扩 `doctor`、`status --human` 文本、`migrate` 等更细 help 语义
- 这次没有修改 README 或 parity ledger
