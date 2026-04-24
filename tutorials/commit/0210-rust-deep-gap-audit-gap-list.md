# 背景

这次提交不是继续补 Rust 功能，而是执行一轮更深的 Rust/Python 语义审核。

前面的对齐工作已经把大面上的结论固定下来了：

1. Python CLI surface 是 Rust CLI 的子集
2. Python MCP surface 是 Rust MCP 的子集
3. 一部分代表性非 CLI 行为已经有 focused parity tests

但这些结论还不足以回答一个更具体的问题：

`Rust 重写和 Python 版本之间，剩下的深层边缘语义差距到底还有哪些？`

所以这次提交的目标不是“再猜一轮缺口”，而是把 gap 审核本身做成 durable artifact。

# 主要目标

这次提交有三个直接目标：

1. 写出一份 `docs/rust-python-deep-gap-audit.md`，把各能力家族逐项审计清楚
2. 从审计报告里提炼出一份只保留 confirmed gaps 的 `docs/rust-python-deep-gap-list.md`
3. 把“哪些不是 gap、哪些是 intentional divergence、哪些已经被 parity tests 锁住”也明确写下来，避免后续重复返工

这次关注的不是 public surface 有没有命令，而是更深的行为层：

1. `layers / maintenance`
2. `registry / KG / diary / manual ops`
3. `conversation mining / general extractor / read-side`
4. `normalize / split`
5. `CLI / MCP` 的 payload / error / path-shape 细节

# 改动概览

本次提交只改 3 个文件：

1. `docs/rust-python-deep-gap-audit.md`
2. `docs/rust-python-deep-gap-list.md`
3. `tutorials/commit/0210-rust-deep-gap-audit-gap-list.md`

其中可以把结果分成两层理解。

第一层是完整审计报告：

1. `docs/rust-python-deep-gap-audit.md` 记录每个能力家族的对照结果
2. 每个结论都必须落到 `confirmed gap`、`intentional divergence`、`not a gap`、`already covered by parity tests` 四种之一
3. 每一行都要求有 Python 侧和 Rust 侧的源码 / 测试证据

第二层是精简 gap 列表：

1. `docs/rust-python-deep-gap-list.md` 只保留 confirmed gaps
2. 这份列表更适合后续开 residual parity batch
3. 它不再混入 supersets、intentional divergence 或已关闭项

这次收口后的关键信息是：

1. 之前已经发现的 `layers / maintenance`、`registry / KG`、`CLI / MCP` 缺口仍然成立
2. `conversation mining / general extractor / split` 这一家族没有再发现新的 confirmed gap
3. `normalize` 家族新增 1 个真正的 confirmed gap：
   Python 对 malformed `.json` / `.jsonl` 会回退到 raw content，Rust 现在会返回 `None`，导致文件被直接跳过

# 关键知识

## 1. 什么叫 deep semantic gap

这里说的 gap，不是“Rust 少了一个命令”这种表面差异。

更典型的是这种情况：

1. CLI / MCP 名字已经有了
2. 看起来功能也能跑
3. 但某个边缘输入下，Python 和 Rust 对同一份数据的处理语义不同

这类差异往往更危险，因为它们不会立刻在 help surface 上暴露出来，却会影响真实用户数据。

## 2. 为什么 `.json/.jsonl` normalize fallback` 是真 gap

Python 的 `normalize()` 逻辑是：

1. 如果像 JSON，就先尝试 `_try_normalize_json`
2. 如果识别失败，不报废文件
3. 最后回退成 plain text 原文返回

Rust 现在的 `normalize_conversation()` 则是：

1. 对 `.json` / `.jsonl` 也先尝试 JSON normalization
2. 但如果 schema 没匹配上，就直接 `Ok(None)`
3. 上层 `mine_conversations_run()` 拿到 `None` 后会 `continue`

这意味着同一份“长得像 JSON、但不是已知 schema”的导出文件，在 Python 会被当 plain text 继续挖，在 Rust 会直接跳过。

这不是风格差异，而是 ingest 语义差异，所以必须列入 confirmed gap。

## 3. 为什么很多项要写成 closed，而不是继续挂 remaining

deep audit 最容易犯的错误，是把“没有重新证明一遍”的东西全部继续挂成 remaining。

这次刻意做了反过来的纪律：

1. 已有 focused parity tests 锁住的，写 `already covered by parity tests`
2. 语义不同但已经接受的，写 `intentional divergence`
3. Python 的 bug class 在 Rust 架构里不再成立的，写 `not a gap`

这样后面的 residual parity batch 才不会反复重开已经审完的老问题。

# 补充知识

## 1. gap list 和 audit report 不该混成一个文件

这次专门拆成两个文档，是因为它们服务的场景不同：

1. `audit.md` 适合复盘“为什么这个结论成立”
2. `gap-list.md` 适合拿来直接排后续实现批次

如果把两者混在一起，最后通常会变成既不利于阅读，也不利于执行。

## 2. 审核时最好把“关闭项”也写出来

只记录发现的问题还不够。

对长期重写项目来说，`closed during audit` 同样重要，因为它能告诉后续 agent：

1. 哪些点已经被审过
2. 哪些差异是有意保留
3. 哪些家族暂时不用再重复盘

这能显著减少“按过时假设重复推进”的风险。

# 验证

这次提交是 docs-only，但做过源码交叉检查和文档校验：

```bash
git -C /Users/dev/workspace2/agents_research/mempalace diff --check -- docs/rust-python-deep-gap-audit.md docs/rust-python-deep-gap-list.md
git -C /Users/dev/workspace2/agents_research/mempalace diff -- docs/rust-python-deep-gap-audit.md docs/rust-python-deep-gap-list.md
```

另外，本轮审计结论依赖的代表性 Rust 证据来自仓库中已经存在并在前序对齐工作中使用过的测试族，例如：

```bash
cd rust && cargo test --test parity_convo_behavior --quiet
cd rust && cargo test --test service_integration service_mine_convos_skips_meta_json_symlink_and_large_files --quiet
cd rust && cargo test --test cli_integration cli_split_writes_files_and_renames_backup --quiet
```

本轮我还尝试重新触发这几个 target，但本地长时间卡在 `cargo` test target 编译 / 锁等待阶段，所以这次提交没有把“重新跑通它们”当成完成条件；这不影响 docs-only 审计结论，因为文档本身来自源码与现有测试的交叉核对。

# 未覆盖项

这次提交刻意没有做下面这些事：

1. 没有修改任何 `rust/src/` 或 `python/mempalace/` 实现代码
2. 没有更新 `docs/parity-ledger.md`，因为这次目标是产出 deep gap artifacts，不是改总账口径
3. 没有把 `docs/superpowers/plans/2026-04-25-rust-python-deep-semantic-gap-audit.md` 混入 durable 提交
4. 没有处理工作区里与本任务无关的 `python/uv.lock`
5. 没有直接修复 gap list 里的 confirmed gaps；后续应按 capability family 单独开 residual parity batch
