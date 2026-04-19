# 背景

前面已经把 transcript normalization 从 `convo.rs` 拆走了，但 `rust/src/convo.rs` 里仍然塞着三类不同职责：

- conversation file scanning
- exchange chunking / room routing
- general memory extraction / marker scoring / sentiment 处理

这会让后续继续调整任一条 conversation ingest 语义时，很容易把另外两条逻辑一起拖进改动范围。

# 主要目标

把 Rust conversation ingestion 内部继续按职责切开，同时保持外部 API 不变：

- `crate::convo::*` 继续是外部统一入口
- `miner` 和现有测试不需要跟着改调用路径
- 行为面保持不变，只收紧内部结构

# 改动概览

这次新增了三个内部模块：

- `rust/src/convo_scan.rs`
- `rust/src/convo_exchange.rs`
- `rust/src/convo_general.rs`

并把 `rust/src/convo.rs` 收成了一个薄 facade。

## 1. `convo_scan`

这里现在承接：

- conversation file scanning
- include override 归一化
- `.meta.json` / extension / symlink / 大文件过滤

也就是“哪些文件能进入 convos ingest”这层。

## 2. `convo_exchange`

这里现在承接：

- `exchange_rooms()`
- `extract_exchange_chunks()`
- `detect_convo_room()`
- quote/speaker/paragraph fallback chunking

也就是“对话交换式 chunk”这一层。

## 3. `convo_general`

这里现在承接：

- `general_rooms()`
- `extract_general_memories()`
- prose extraction
- marker scoring
- sentiment / resolution / disambiguation

也就是“general memory extraction”这一层。

## 4. `convo`

这里现在只保留：

- `ConversationChunk`
- `MIN_CONVO_CHUNK_SIZE`
- 对三个内部模块的统一 re-export

这样外部仍然可以继续写：

```rust
use mempalace_rs::convo::{
    ConversationChunk,
    detect_convo_room,
    extract_exchange_chunks,
    extract_general_memories,
    general_rooms,
    exchange_rooms,
    scan_convo_files,
};
```

而不用感知内部模块已经拆开。

# 关键知识

## 1. facade 的作用是稳住上层依赖

这次没有让 `miner.rs`、测试、或者 service 调用方直接改成依赖：

- `convo_scan::*`
- `convo_exchange::*`
- `convo_general::*`

而是继续通过 `convo.rs` 统一 re-export。

这样以后即使还要继续细拆 `exchange` 或 `general` 内部 helper，也不会让上层 import 路径跟着反复变化。

## 2. conversation ingest 的三层变化节奏不同

这三层看起来都属于“conversation 处理”，但真实维护节奏并不一样：

- 扫描规则：更接近文件系统/ignore 行为
- exchange chunking：更接近 transcript 结构 heuristics
- general extractor：更接近语义分类 heuristics

把它们放进一个文件里，任何一类改动都容易制造无关 diff。拆开之后，每层可以独立演进。

# 补充知识

## 为什么 `ConversationChunk` 还留在 `convo.rs`

`ConversationChunk` 是这三层共享的公共结果类型：

- `exchange` 会产出它
- `general` 会产出它
- 上层 `miner` 也直接消费它

所以让它继续挂在 facade 上最稳妥，避免把公共类型误放进某一个子模块后造成依赖方向混乱。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

验证通过，说明这次 conversation 内部切分没有改变现有 convos ingest 的外部行为。

# 未覆盖项

这次没有继续改：

- `miner.rs`
- `normalize.rs`
- `spellcheck.rs`

因为目标只是把 `convo.rs` 内部职责拆开，而不是继续收口更上层的 mining orchestration。
