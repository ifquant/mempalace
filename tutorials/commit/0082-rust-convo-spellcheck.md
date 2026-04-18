## 背景

Python 版在 conversation normalize 里有一层很实际的小能力：在把聊天导出转成 transcript 时，会对 user turn 做 best-effort spellcheck。

这层能力看起来小，但它会直接影响：

- exchange chunk 的质量
- general extractor 的 marker 命中率
- 后续 search / compress / wake-up 对用户原意的恢复程度

Rust 之前已经能 normalize 各种 chat export，但还没有把这层 spellcheck 接进去。

## 主要目标

- 给 Rust 增加本地 spellcheck 模块
- 只修 user turn，不改 assistant turn
- 保留 known names、CamelCase、技术 token、URL/path
- 把 spellcheck 真正接进 convo normalize，而不是做孤立工具

## 改动概览

- 新增 [rust/src/spellcheck.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/spellcheck.rs)
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs) 导出 spellcheck 模块
- 更新 [rust/src/convo.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/convo.rs)
  - `normalize_conversation_file()` 会按路径加载附近的 `entity_registry.json`
  - `>` transcript 走 `spellcheck_transcript()`
  - JSON / JSONL chat export 在 `messages_to_transcript()` 里只修 user turn
- 更新 [rust/tests/service_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/service_integration.rs)
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. spellcheck 最好挂在 normalize 入口，而不是挖掘后补丁

如果 spellcheck 放到 chunk 之后，问题会变复杂：

- exchange/general 两条路都得各自修
- chunk 边界前后的上下文可能已经丢失
- 同一份 transcript 在不同 extract 模式下可能出现不一致

所以这轮选择在 normalize 阶段做：

- quote transcript：只修 `>` 行
- JSON / JSONL export：只修 user message

这样后面的 exchange/general 都天然吃到同一份更干净的 transcript。

### 2. 保护名字和技术词，比“尽量多纠错”更重要

MemPalace 这类系统最怕的是把：

- `Riley`
- `MemPalace`
- `ChromaDB`
- `bge-large-v1.5`

这种词误改掉。

所以这轮 spellcheck 的策略不是“激进纠错”，而是“保守纠错”：

- known names 不动
- CamelCase 不动
- ALL_CAPS 不动
- 含数字、连字符、下划线的不动
- URL / path 不动
- Capitalized token 默认不动

真正会修的是：

- 小写 flowing text
- 常见 typo map
- 少量系统词典 edit-distance 修正

### 3. nearby registry 比 global registry 更符合当前 Rust 路线

Python 版很多逻辑默认从 `~/.mempalace` 出发。  
Rust 现在的路线更明确：project-local / palace-local 优先。

所以这里不是去读全局 registry，而是从 transcript 路径往上找附近的 `entity_registry.json`。

这样做的好处是：

- 对本地项目 chat export 更自然
- 和 Rust 当前 project-local bootstrap 保持一致
- 不把不同项目的人名上下文混在一起

## 补充知识

### 为什么没直接接外部 autocorrect 库

Python 版的 spellcheck 依赖 `autocorrect`，但 Rust 这边如果直接拉一套新的重依赖，并不划算。

这轮先走了轻量策略：

- 常见 typo map
- `/usr/share/dict/words`
- 受限 edit distance 搜索

这能把主路径先打通，而且不引入太重的运行时负担。

### 为什么只修 user turn

assistant turn 是历史记录的一部分，贸然改写它会带来两个风险：

- 失真原始会话
- 影响以后做 verbatim retrieval / audit

所以这轮严格保持 Python 的方向：只修 user turn。

## 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 还没有把 spellcheck 暴露成独立 CLI / MCP 工具
- 还没有引入更强的 language model / frequency dictionary 来做更高级纠错
- 目前只从 nearby `entity_registry.json` 取 known names，没有并入全局 palace facts
- 还没有覆盖所有 Python spellcheck 里的长尾 typo 行为
