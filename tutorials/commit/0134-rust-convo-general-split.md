# 背景

前几轮已经把 `convo.rs` 拆成：

- `convo_scan`
- `convo_exchange`
- `convo_general`

但 `rust/src/convo_general.rs` 里仍然同时装着两类不同节奏的逻辑：

- transcript segmentation / prose extraction
- marker scoring / sentiment / resolution disambiguation

这会让后续如果只想改 general extractor 的 scoring heuristic，也必须翻同一个文件里的 turn grouping 和 paragraph fallback；反过来如果只是改 segmentation，也会把一整套 marker table 和 sentiment 判别一起带进 diff。

# 主要目标

把 Rust general-memory extractor 再按职责切开，同时保持外部 API 不变：

- `general_rooms()` 继续可用
- `extract_general_memories()` 继续可用
- CLI / service / convo mining 调用方不需要改 import 路径

# 改动概览

这次新增了两个内部文件：

- `rust/src/convo_general_segments.rs`
- `rust/src/convo_general_scoring.rs`

并把 `rust/src/convo_general.rs` 收成 public extraction facade。

## 1. `convo_general_segments`

这里现在承接：

- `split_into_segments()`
- `extract_prose()`
- paragraph fallback
- turn-marker counting
- speaker-role detection
- code-line filtering

也就是 general extractor 里“如何把一份 transcript 切成候选语义片段”的那部分。

## 2. `convo_general_scoring`

这里现在承接：

- decision / preference / milestone / problem / emotional marker tables
- `score_segment()`
- `confidence()`
- sentiment 判别
- resolution 判别
- `problem -> milestone / emotional` 的 disambiguation

也就是 general extractor 里“如何给一个 prose segment 归类”的那部分。

## 3. `convo_general`

这里现在只保留：

- `GENERAL_TYPES`
- `general_rooms()`
- `extract_general_memories()`

换句话说，它现在只负责把：

- segment provider
- scoring provider

串起来，然后组装 `ConversationChunk`。

# 关键知识

## 1. segmentation 和 scoring 的变化节奏不同

这两层都属于 general extractor，但维护节奏不一样：

- segmentation 更容易因为 transcript 格式、turn marker、paragraph fallback 而调整
- scoring 更容易因为 marker、confidence、sentiment 或 resolution heuristic 而调整

把它们混在一个文件里，会让任何一侧的小调优都制造跨职责 diff。拆开之后，review 边界更清楚。

## 2. 提取循环最好只负责 orchestration

`extract_general_memories()` 这种 public entrypoint 最合适的形态，不是自己携带所有 regex / marker / segmentation 细节，而是：

- 调 segmenter
- 调 scorer
- 过滤 low-confidence
- 组装输出

这样上层一眼就能读懂“这个 extractor 在干什么”，而底层 heuristics 则可以分别迭代。

# 补充知识

## 为什么 `GENERAL_TYPES` 还留在 `convo_general.rs`

`GENERAL_TYPES` 是这个 public extractor surface 的一部分，它描述的是：

- general extractor 对外暴露哪些 memory room

它更像 facade 的 contract，而不是 segment 或 scoring 的内部实现细节，所以这次继续留在 `convo_general.rs` 更稳。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

验证通过，说明这次 general extractor internal split 没有改变外部 `extract_general_memories()` 行为，也没有破坏现有 convo mining 回归。

# 未覆盖项

这次没有继续改：

- `convo_exchange.rs`
- `normalize.rs`
- `miner_convo.rs`
- `spellcheck.rs`

因为目标只是把 `convo_general.rs` 内部再按 segment/scoring 职责切开，而不是继续扩散到整个 convo pipeline。
