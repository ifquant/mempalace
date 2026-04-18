## 背景

前面 Rust 版已经把不少 Python 对应模块抽成了独立库层：

- `palace`
- `layers`
- `searcher`
- `entity_detector`
- `room_detector`

但 transcript normalization 还停在旧状态：能力已经有了，却还是埋在
[rust/src/convo.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/convo.rs) 里面。

这会导致两个问题：

1. `normalize <file>`、MCP `mempalace_normalize`、convo miner 实际上都在用同一条能力，但模块边界看不出来  
2. Rust 的库层 shape 还没有真正对齐 Python `normalize.py`

所以这一轮要把 normalization 独立出来，让 `convo` 只负责后续 chunking / extraction。

## 主要目标

- 给 Rust 新增独立 `normalize` 模块
- 把 transcript normalization 从 `convo.rs` 里抽出来
- 保持现有支持格式和行为不变
- 让 CLI / MCP / service 改走新模块
- 把这层库 API 写进 README

## 改动概览

- 新增 [rust/src/normalize.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/normalize.rs)
  - `normalize_conversation_file()`
  - `normalize_conversation()`
  - 内部 JSON / JSONL parser 与 transcript renderer
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `normalize`
- 更新 [rust/src/convo.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/convo.rs)
  - 删除内嵌 normalization 逻辑，保留 chunking / extraction
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - convos mining 改从 `normalize` 模块取 transcript
- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - `normalize <file>` 改用新模块
- 更新 [rust/src/mcp.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/mcp.rs)
  - `mempalace_normalize` 改用新模块
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. normalize 和 convo 不是同一个职责

这两层经常连在一起出现，但职责不一样：

- `normalize`
  - 识别输入格式
  - 把各种 chat export 变成统一 transcript
  - 处理 spellcheck / user turn formatting
- `convo`
  - 在统一 transcript 之上做 exchange/general 抽取
  - 做 room 检测、memory type 提取、chunking

如果继续把 normalize 放在 `convo.rs` 里，模块名会越来越不准确，库调用方也很难只拿到“标准化”这一步。

### 2. 这类抽取要避免语义顺手改动

这一轮没有去改：

- ChatGPT JSON 解析规则
- Claude/Codex JSONL 识别规则
- Slack transcript 转换规则
- quote transcript spellcheck 规则

只做边界重组，不顺手改语义，风险最低，也最容易靠现有回归证明“行为没漂”。

## 补充知识

### 1. 库层对齐比 CLI 对齐更难，也更值钱

CLI 看起来对齐，很多时候只是“能跑”。真正让后续 agent 和调用方受益的是：

- 能不能按 Python 一样找到对应模块
- 能不能按模块职责直接复用
- 能不能避免 service/CLI/MCP 各自维护一份逻辑

所以这几轮虽然看起来都像“抽文件”，其实是在把 Rust 版从“功能集合”收成“可持续维护的库层”。

### 2. transcript 标准化和 spellcheck 适合一起留在 normalize 层

因为 spellcheck 只在“把输入转成 transcript”这一步最自然：

- 原始 JSON/JSONL 里 user turn 还没被统一出来
- 进入 convo extractor 之后，又已经太晚了

所以把它和 transcript renderer 放在同一层，比挂在 `convo` 里更顺手。

## 验证

实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

本次新增/保留覆盖了：

- Codex JSONL transcript normalize
- ChatGPT JSON transcript normalize
- 原有 convo mining / CLI normalize / MCP normalize 路径继续通过

## 未覆盖项

- 这次没有新增独立 `Normalizer` facade struct，只先把模块边界立住
- 这次没有扩新格式支持，仍然只保持现有 transcript 输入面
- 这次没有改 Python `normalize.py`，只是把 Rust 的库层形状继续往它对齐
