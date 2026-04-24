# 背景

前面的 5 个 residual parity 提交已经分别补上了 `Layer1`、`repair_prune`、`registry`、`knowledge graph`、`CLI/MCP/normalize` 这几组真实差距，但仓库里的 durable 文档还停留在“有 11 个 confirmed gaps”等待处理的状态。

这会带来一个很实际的问题：代码已经对齐了，但后续 agent 或新同学再看 `docs/rust-python-deep-gap-list.md`、`docs/rust-python-deep-gap-audit.md`、`docs/parity-ledger.md`、`rust/README.md` 时，仍然会以为这些缺口还没补完，然后继续按过时假设推进。

# 主要目标

把 residual parity 收口后的真实状态写回 durable 文档，并补一轮完整 Rust 验证，确保这条线从“实现完成”推进到“文档与验证也完成”。

# 改动概览

- 更新 `docs/rust-python-deep-gap-list.md`
  - 把 `Confirmed Gaps` 清空为 `None currently`
  - 把之前的 11 个 gap 迁移到 `Closed During Audit`
- 更新 `docs/rust-python-deep-gap-audit.md`
  - 把审计定位从“只做 gap 发现”改成“gap 审计 + residual parity closeout”
  - 增加 closeout update，总结 5 个 capability family 已经关闭的具体缺口
  - 把 summary 改成 “confirmed gaps remaining: none currently”
- 更新 `docs/parity-ledger.md`
  - 在 snapshot 中明确 `deep-gap-list` 已无 confirmed remaining gaps
  - 把 layers/maintenance、registry/KG、CLI/MCP/normalize 的 closeout 结果写入 completed behavior audits
- 更新 `rust/README.md`
  - 同步写明 residual deep-gap parity batches 已关闭
  - 指向 `docs/rust-python-deep-gap-list.md`
- 修正验证阶段暴露的收尾问题
  - `rust/tests/cli_integration.rs` 里剩余 3 个 schema 版本硬编码断言改为 `CURRENT_SCHEMA_VERSION`
  - `rust/tests/mcp_integration.rs` 把 `mempalace_normalize` 的断言改成新的 raw-fallback 语义
  - `rust/tests/service_integration.rs` 把 convo mining 断言改成兼容 `entity_registry.json` 也会被 raw-fallback ingestion 的现状
  - `rust/src/mcp_schema_support.rs` 去掉一个 clippy 报出的显式解引用

# 关键知识

`gap list` 和 `parity ledger` 的职责不一样：

- `docs/rust-python-deep-gap-list.md` 只列“当前仍然存在的 confirmed gaps”
- `docs/parity-ledger.md` 则是更高层的状态总账，负责说明哪些面已经 `aligned`、哪些是 `rust superset`、哪些属于 `intentional divergence`

所以 closeout 时不能只改一个文件。只改 `gap list` 会让高层总账失真；只改 `ledger` 又会让审计证据文件继续误报残项。

完整验证也不能省：

- 前面 5 笔提交都只跑了局部测试
- closeout 这一笔需要确认整棵 `rust/` 在当前累计状态下仍然能过 `fmt/check/test/clippy`
- 全量验证经常会暴露“功能已对，但测试锚点还停留在旧版本假设”的问题，这类问题也应该在 closeout 提交里一起收掉

# 补充知识

1. 文档 closeout 不是“写个总结”这么简单，而是把不同层级的事实来源同步起来。这里至少有审计报告、gap list、ledger、README 四层，它们如果不同步，后面 agent 很容易重复开工。

2. 当一个多提交执行计划最后一笔是 `docs closeout + full verification` 时，验证里抓出来的“旧断言”“lint 残差”“新语义下的测试预期漂移”都要一起处理。否则你会看到实现已经对齐，但全量验证仍然因为旧测试或 lint 锚点而不绿。

# 验证

```bash
cd /Users/dev/workspace2/agents_research/mempalace
git diff --check -- docs/rust-python-deep-gap-audit.md docs/rust-python-deep-gap-list.md docs/parity-ledger.md rust/README.md rust/src/mcp_schema_support.rs rust/tests/cli_integration.rs rust/tests/mcp_integration.rs rust/tests/service_integration.rs tutorials/commit/0217-rust-residual-parity-closeout.md

cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 没有改 `python/` 实现，也没有改变 Python 当前的行为基线
- 没有把 Rust 宣布为默认入口；`Python palace data compatibility` 仍然是 ledger 里明确保留的 `intentional divergence`
- 没有修改 `docs/superpowers/` 下的计划文件
