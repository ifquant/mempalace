# 0156 Rust `registry` / `normalize` help 覆盖补齐

## 背景

前两轮已经在收 Rust CLI 的 help/test 一致性，但还有两个很容易漏掉的入口：

- `registry --help`
- `normalize --help`

它们本身已经工作正常，但测试覆盖还没有把当前真实 help 面锁紧：

- `registry --help` 只锁了 `summary/lookup/learn/research/confirm`
- `normalize --help` 只锁了标题，没有锁 `<FILE>` 参数描述

这类缺口不会让实现坏掉，但会让后续 help 文案漂移时更难第一时间发现。

## 主要目标

把 `registry --help` 和 `normalize --help` 的测试断言补到与当前真实输出一致。

## 改动概览

- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - `cli_registry_help_mentions_summary_lookup_learn_and_research`
    - 补锁 `add-person`
    - 补锁 `add-project`
    - 补锁 `add-alias`
    - 补锁 `query`
  - `cli_normalize_help_mentions_chat_export_normalization`
    - 补锁 `<FILE>` 参数描述
- 新增本教程文档

## 关键知识

### 1. 帮助文本的“命令列举”比标题更容易漂移

标题通常很稳定，但像：

- 某个子命令列表
- 某个位置参数说明

更容易在后续改动时被顺手改掉，所以测试更应该锁这些细节。

### 2. `registry` 是高频但容易漏测的聚合入口

`registry` 下面有多条子命令，覆盖不完整时，很容易只测“最早那几条”，后来加的写入/query 分支就没人盯了。  
这次补的是这个聚合入口的 help 覆盖，不是实现逻辑本身。

### 3. 参数描述也是用户接口的一部分

`normalize` 的 `<FILE>` 描述虽然不是 flag，但它直接影响用户对输入格式的理解。  
锁住这类参数说明，能让 CLI 更像稳定接口，而不是“看代码才知道怎么传”。

## 补充知识

### 1. 一致性收口最好优先补“聚合入口”

像 `registry --help` 这种总入口，一条测试就能覆盖很多子命令名，比逐个子命令补 help 测试更划算。

### 2. 轻量测试补齐适合放在连续小切片里

这类提交范围很窄：

- 不改实现
- 不碰存储
- 风险低

但对后续持续演进很有价值，所以很适合在“继续收口”阶段连续推进。

## 验证

在 `rust/` 目录执行：

```bash
cargo fmt --check
cargo test cli_registry_help_mentions_summary_lookup_learn_and_research -- --exact
cargo test cli_normalize_help_mentions_chat_export_normalization -- --exact
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这次没有修改任何 `rust/src/` 实现逻辑
- 这次没有继续扩 `repair`、`search`、`mine` 等其它子命令的 help 断言
- 这次没有更新 README，因为本轮只是在锁 help 覆盖，而不是调整示例或表述
