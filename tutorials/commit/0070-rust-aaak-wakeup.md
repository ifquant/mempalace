# 背景

Rust 版 `mempalace` 前面已经把 `mine/search/status/mcp` 主链路拉起来了，也补了 `convos/general`。但 README 里还留着一个明显缺口：Python 版有 `compress` 和 `wake-up`，Rust 版还没有真正的 AAAK 汇总层和唤醒上下文。

这会带来两个问题：

1. Rust 只能直接读 drawer 原文，缺少一层更便宜的摘要表示。
2. Rust 还不能像 Python 一样快速生成 `L0 + L1` 的 wake-up 上下文。

所以这一提交的目标不是继续扩 MCP，而是把 AAAK / wake-up 这条高层能力补成一个真正可运行、可持久化、可测试的 Rust 闭环。

# 主要目标

- 给 Rust 增加 AAAK dialect 核心模块。
- 新增 `compress` CLI，能从现有 drawers 生成 AAAK summary 并写入 SQLite。
- 新增 `wake-up` CLI，能输出 palace-local 的 `identity.txt` 和 L1 essential story。
- 把 SQLite schema 升到 `v7`，正式持久化 `compressed_drawers`。
- 补齐 service / CLI / migration 测试，并同步 README。

# 改动概览

- 新增 [rust/src/dialect.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/dialect.rs)
  - 提供共享 `AAAK_SPEC`
  - 提供 `Dialect::compress()`、`compression_stats()`、`count_tokens()`
  - 先按 Python 版的轻量规则做实体、topic、key sentence、emotion、flag 提取
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `dialect` 模块
- 更新 [rust/src/storage/sqlite.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/storage/sqlite.rs)
  - schema version 升到 `7`
  - 新增 `compressed_drawers` 表
  - 新增 `migrate_v6_to_v7()`
  - 新增 drawer 读取、recent drawer 读取、compressed drawer 替换/列出接口
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - 新增 `App::compress()`
  - 新增 `App::wake_up()`
  - 新增 palace-local identity 读取和 L1 渲染逻辑
- 更新 [rust/src/config.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/config.rs)
  - 新增 `identity_path()`，固定使用 `<palace>/identity.txt`
- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - 新增 CLI 子命令：
    - `compress`
    - `wake-up`
  - 两者都支持默认 JSON 输出和 `--human`
  - 补了对应 no-palace / error path
- 更新 [rust/src/mcp.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/mcp.rs)
  - 共享 `dialect::AAAK_SPEC`，不再复制常量
- 更新测试：
  - [rust/tests/service_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/service_integration.rs)
  - [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
- 更新文档：
  - [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 1. AAAK 在 Rust 里先做“稳定摘要层”，不是做无损压缩

Python `dialect.py` 虽然叫 compress，但它本质不是 zip/gzip 那种无损压缩，而是：

- 从原文里抽出更重要的结构信息
- 生成一个 LLM 也能直接读的紧凑摘要

所以 Rust 这里不要把它实现成字节压缩器，而是要实现成：

- `compress(text, metadata) -> aaak_text`
- 再额外算统计值 `original_tokens / compressed_tokens / ratio`

这样 `compress` 命令和 `wake-up` 才都能复用这层表示。

## 2. `wake-up` 不一定要先做完整 Layer 系统，先把最小闭环做实

Python 的 `layers.py` 有 L0 / L1 / L2 / L3 概念，但当前 Rust 最需要的是：

- L0 identity
- L1 essential story

所以这一轮不去过度设计多层框架，而是先让：

- `<palace>/identity.txt`
- `recent_drawers -> grouped summary`

形成一个稳定输出。这样用户已经能实际用起来，后面再扩 L2/L3 也更自然。

## 3. schema 迁移最好和“新能力的真实持久化表”一起落地

如果只加 CLI、不加持久化，很快就会出现“命令能跑，但状态没地方存”的半成品。

这里直接把 `compressed_drawers` 落到 SQLite，并把 schema 升到 `v7`，好处是：

- `compress` 结果有正式归宿
- `migrate` 也能覆盖这条新能力
- 测试可以直接查库验证，不用靠 stdout 猜测

# 补充知识

## 1. “压缩率一定 > 1” 这种断言在真实工程里不稳

实现后测试一开始用的是“短文本也应该有明显压缩率”。这在 demo 里很诱人，但对短内容并不稳定，因为：

- header 本身就有固定开销
- 很短的文本本来就没多少可折叠空间

所以更稳的测试方式是断言：

- AAAK 文本真的生成了
- token 统计存在
- 结果确实被落盘

不要把不稳定的经验值写成硬规则。

## 2. palace-local 路径比全局 home 路径更适合 Rust 这条重写线

Python 的 `Layer0` 默认读 `~/.mempalace/identity.txt`。Rust 这次没有照搬，而是改成：

- `<palace>/identity.txt`

原因是 Rust 重写线一直在强调 local-first 和 palace self-contained：

- 一个 palace 目录应该尽量自带自己的上下文
- 这样测试、迁移、备份都更清晰

这也是“看起来和 Python 不完全一样，但更符合 Rust 当前路线”的典型例子。

# 验证

本次实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

其中新增覆盖包括：

- `cli_compress_help_mentions_human_output`
- `cli_wake_up_help_mentions_human_output`
- `cli_compress_json_stores_aaak_summaries`
- `cli_wake_up_human_prints_identity_and_layer1`
- `migrate_v6_adds_compressed_drawers_table`
- `compress_stores_aaak_summaries_and_wake_up_uses_identity`

# 未覆盖项

- 还没把 `compress` / `wake-up` 接进 MCP；这轮只做 CLI + service + SQLite 持久化。
- 还没实现 Python 更完整的 `Layer2 / Layer3` 加载体系。
- 还没做 AAAK 的更复杂 tunnel / arc / zettel 图结构，只做了当前最小可用的 structured summary。
- 还没实现 Python 的全局 `~/.mempalace/identity.txt` 兼容；Rust 当前明确走 palace-local `identity.txt`。
