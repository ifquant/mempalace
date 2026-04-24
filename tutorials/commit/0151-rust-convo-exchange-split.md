# 0151 Rust `convo_exchange` 内部继续拆分

## 背景

前面已经把 Rust 的 conversation ingest 主干拆成了 `convo_scan`、`convo_exchange`、`convo_general` 三块，但 `convo_exchange.rs` 里仍然同时塞着两类不同职责：

- 一类是“这个对话应该进哪个 room”的关键词桶和判定逻辑
- 另一类是“怎么把 transcript 切成 exchange chunk”的 quote/speaker/paragraph 启发式

这会让后续继续对齐 Python 行为时，房间路由和 chunking 规则彼此缠住，文件也重新开始变大。

## 主要目标

把 `convo_exchange.rs` 再往下按职责切开，同时保持外部 `crate::convo::*` 和 `extract_exchange_chunks()` / room detection 的现有调用面不变。

## 改动概览

- 新增 `rust/src/convo_exchange_rooms.rs`
  - 承载 `TOPIC_BUCKETS`
  - 承载 `exchange_rooms()`
  - 承载 `detect_convo_room()`
- 新增 `rust/src/convo_exchange_chunking.rs`
  - 承载 exchange chunking 的主入口 `extract_exchange_chunks()`
  - 承载 quote-line / speaker-turn / paragraph fallback 相关 helper
- 精简 `rust/src/convo_exchange.rs`
  - 改成薄 facade
  - 只负责内部模块声明、public re-export、以及这组行为的测试锚点
- 更新 `rust/README.md`
- 新增本教程文档

## 关键知识

### 1. room routing 和 chunking 是两种不同变化轴

这两块都会影响“从 transcript 提取什么 drawer”，但它们的演进方向不一样：

- room routing 关心语义分类和关键词桶
- chunking 关心结构边界和切分粒度

把它们拆开后，后面如果只想调 room 命中规则，不需要冒着碰坏 chunking 的风险一起改。

### 2. facade 文件的价值是稳住上层调用面

这次没有让上层改成直接引用新文件，而是保留 `convo_exchange.rs` 作为薄入口。这样：

- `lib.rs` 和其它调用者不用跟着大面积改 import
- 测试锚点还能集中放在 facade 层
- 后续继续拆内部时，外部 API 仍然稳定

### 3. paragraph fallback 仍然是 exchange 路径的一部分

即使 transcript 没有清晰 speaker marker，这条链路仍然会尝试按段落分组形成 exchange chunk。这个 fallback 不能在拆文件时丢掉，否则会直接回退到更弱的 general extraction 路径，行为会和之前不一致。

## 补充知识

一个实用判断标准是：如果两个逻辑块共享输入文件，但“测试关注点”和“调参方向”明显不同，就值得拆。

在这里：

- room detection 测的是关键词桶是否把内容路由到正确 room
- chunking 测的是 transcript 边界、配对、fallback 是否稳定

这就是很典型的“同输入，不同关注点”。

## 验证

在 `rust/` 目录顺序执行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这次没有改 Python `python/` 侧的 conversation ingest 实现
- 这次没有扩展新的 exchange room bucket 语义，只做 Rust 内部职责拆分
- 这次没有继续拆 `convo_scan` 或 `convo_general` 之外的更深层 helper
