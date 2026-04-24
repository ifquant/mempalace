# 背景

这一提交对应的是 Rust 重写线的一次“代表性行为对齐收口”。

在这之前，仓库已经有 `docs/parity-ledger.md`，并且 CLI / MCP 的公开表面能力基本已经盘清了：Python CLI 是 Rust CLI 的子集，Python MCP 也是 Rust MCP 的子集。真正还没有被稳定锁住的，是更深一层的行为语义。

这次改动的目标不是再扩一大批功能，而是做三件更扎实的事：

1. 为三组关键行为家族补 focused parity tests
2. 让测试真正打出 Rust 当前还存在的行为漂移
3. 把 ledger / README 改成与证据一致的表述

# 主要目标

这次提交的主要目标是把 Rust 重写里的“代表性非 CLI 行为”锁成可验证状态。

具体分成三组：

1. `layers / maintenance`
2. `registry / KG / read-diary`
3. `conversation mining / read-side`

这里故意使用“representative behavior”而不是“整类全面完成”，因为本次证据来自 focused parity tests，不是把所有边缘分支都穷尽了一遍。

# 改动概览

本次提交改了 7 个文件：

1. `rust/tests/parity_layers_maintenance.rs`
2. `rust/tests/parity_registry_kg_ops.rs`
3. `rust/tests/parity_convo_behavior.rs`
4. `rust/src/layers.rs`
5. `rust/src/dedup.rs`
6. `docs/parity-ledger.md`
7. `rust/README.md`

可以按两层来理解。

第一层是补测试：

1. `parity_layers_maintenance.rs` 锁住了 Layer 0 identity trim / token estimate、dedup 对短文本的 dry-run 规划、repair prune preview 不落库
2. `parity_registry_kg_ops.rs` 锁住了 KG 写入自动建实体与统计、registry lookup 的上下文消歧、diary 空结果提示
3. `parity_convo_behavior.rs` 锁住了 convo 重挖时旧 source 替换、正向已解决文本不误归到 `problem`、wake-up 的读侧契约

第二层是修真实 drift：

1. `rust/src/layers.rs` 里 `LayerStack::layer0()` 不再用 whitespace count，而是改成 Python 风格的 char-based token estimate
2. `rust/src/dedup.rs` 在规划阶段会把小于 20 字符的 drawer 稳定记入删除计划；dry-run 仍然只生成 summary，不做实际删除

最后一层是文档对齐：

1. `docs/parity-ledger.md` 新增三条 completed behavior audits
2. 删除过于泛化的 `Deeper non-CLI behavior audit` remaining 行
3. `rust/README.md` 改成更精确的 “representative non-CLI behavior cases”

# 关键知识

## 1. 为什么先补 focused parity tests

如果没有 focused tests，很容易出现两种坏情况：

1. 以为功能“差不多了”，但真实语义还漂着
2. 把后续工作继续建立在 README 或口头判断上，而不是建立在固定下来的证据上

这次的做法是先给每个行为家族找一个能代表 Python 契约的测试点。这样以后继续收口时，至少不会把已经对齐的行为又说成“剩余工作”。

## 2. 为什么 Layer 0 的 token estimate 要看字符，不看单词

这次最典型的真实 drift 就是 `layer0.token_estimate`。

Rust 之前的实现相当于“按空白切词计数”，但仓库里通用的 token 估算逻辑是：

```rust
text.chars().count().div_ceil(4)
```

这个差异在普通短文本上不明显，但在长、连续、少空格文本上会立刻暴露出来。测试里用 `400` 个 `A` 就能稳定打出来：正确结果是 `100`，不是 `1`。

## 3. dedup 的 short-doc 规则为什么要在 planning 阶段处理

`dedup` 不是只有“真的删了什么”才重要，`dry_run` 的 summary 也属于用户可见行为。

如果 short-doc 规则只在执行删除时处理，而 planning 阶段不处理，就会出现：

1. dry-run 看起来不会删
2. 真执行时却删了

这会直接破坏 CLI / MCP 的可预期性。所以这次把 short-doc 过滤前移到了 plan 构建阶段。

# 补充知识

## 1. parity ledger 最怕的不是漏项，而是话说太满

做对齐总账时，一个常见错误是把“抽样证明”写成“全面对齐”。

这次 reviewer 的一个关键反馈就是：当前证据只足以支持 “focused representative cases aligned”，还不该写成“这一整类已经完全端到端对齐”。这是写总账时非常重要的纪律。

## 2. 并行跑 Rust 测试时，`CARGO_TARGET_DIR` 很适合隔离

这次 subagent 并行跑测试时，主工作树的 `target/` 很容易因为多个 `cargo` 进程互相抢锁而变慢。

一个很实用的技巧是给不同验证任务单独指定：

```bash
CARGO_TARGET_DIR=/tmp/mempalace-taskX cargo test ...
```

这样不会污染仓库，也能降低并发时的锁竞争。

# 验证

本次实际做过的验证包括：

```bash
git diff --check
cd rust && cargo fmt --check
cd rust && cargo test --test parity_layers_maintenance --quiet
cd rust && cargo test --test parity_registry_kg_ops --quiet
cd rust && cargo test --test parity_convo_behavior --quiet
```

另外，在实现过程中还跑过对应的 focused integration 验证：

```bash
cd rust && cargo test --test service_integration repair_scan_prune_and_rebuild_handle_vector_drift --quiet
cd rust && cargo test --test service_integration dedup_removes_near_identical_drawers_from_same_source --quiet
cd rust && cargo test --test service_integration registry_summary_lookup_and_learn_work --quiet
cd rust && cargo test --test service_integration kg_round_trip_and_taxonomy_work --quiet
cd rust && cargo test --test mcp_integration mcp_registry_tools_work --quiet
cd rust && cargo test --test service_integration service_mine_convos_exchange_replaces_existing_source_chunks --quiet
cd rust && cargo test --test service_integration service_general_extractor_keeps_positive_emotional_text_out_of_problem --quiet
cd rust && cargo test --test service_integration compress_stores_aaak_summaries_and_wake_up_uses_identity --quiet
```

# 未覆盖项

这次提交刻意没有覆盖下面这些东西：

1. 没有把 `docs/superpowers/plans/2026-04-24-rust-python-parity-audit.md` 混入实现提交
2. 没有扩展到 Python 代码改动
3. 没有新增 `.github/workflows/` 改动
4. 没有把“代表性行为对齐”继续夸大成“整类语义完全穷尽”
5. 没有补更广泛的 manual add/delete parity 测试；本次只锁住了 read-diary 空结果这类已审到的代表性行为
