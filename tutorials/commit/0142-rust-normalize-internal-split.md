# 背景

前面已经把 Rust 的 project transcript CLI 收成了：

- `project_cli_transcript_split`
- `project_cli_transcript_normalize`

但它底下真正干活的 `rust/src/normalize.rs` 仍然把两类不同节奏的逻辑堆在一起：

- quote transcript / spellcheck 路径
- JSON / JSONL chat export 解析路径

随着支持的聊天导出格式越来越多，这个文件会越来越像一个“解析器大杂烩”。

# 主要目标

把 Rust normalize internals 再按职责切开，同时保持外部 API 不变：

- `normalize_conversation_file()` 继续可用
- `normalize_conversation()` 继续可用
- CLI / service / miner 调用方不需要改 import 路径

# 改动概览

这次新增了两个内部文件：

- `rust/src/normalize_transcript.rs`
- `rust/src/normalize_json.rs`

并把 `rust/src/normalize.rs` 收成 public normalization facade。

## 1. `normalize_transcript`

这里现在承接：

- `count_quote_lines()`
- `messages_to_transcript()`
- `normalize_quote_transcript()`

也就是 transcript 风格输入的那部分共性逻辑：

- quote-line 识别
- user/assistant transcript 组装
- user turn spellcheck

## 2. `normalize_json`

这里现在承接：

- `try_normalize_json()`
- `try_claude_code_jsonl()`
- `try_codex_jsonl()`
- `try_flat_messages_json()`
- `try_claude_ai_json()`
- `try_chatgpt_json()`
- `try_slack_json()`
- `extract_content()`

也就是所有结构化 chat export 的解析路径。

## 3. `normalize`

这个文件现在只保留：

- `normalize_conversation_file()`
- `normalize_conversation()`
- 顶层“先判 quote transcript，再判 JSON/JSONL，再 fallback”路由

换句话说，它现在只负责 public entrypoint 和总调度，不再承载全部 parser 细节。

# 关键知识

## 1. transcript 路径和 JSON parser 路径的变化来源不同

这两块都属于 normalize，但维护节奏不一样：

- transcript 路径更容易因为 spellcheck、quote-line heuristics、pairing 逻辑而调整
- JSON parser 路径更容易因为某个导出格式字段变化而调整

如果继续放在一个文件里，任何一侧的小改动都会制造跨职责 diff。

## 2. public facade 只保留路由最稳

`normalize_conversation()` 最合适的形状，不是自己携带所有 JSON parser 细节，而是：

- 先处理空内容
- 再判断 quote transcript
- 再判断 JSON/JSONL
- 最后 fallback

这样上层一眼就能看明白 normalize 的总流程，而 parser 细节则各自收口。

# 补充知识

## 为什么 `messages_to_transcript()` 归到 transcript 模块

虽然它也被 JSON parser 调用，但它生成的是 transcript 语义输出，而不是解析 JSON 的一部分。也就是说：

- JSON parser 负责“提取 role/text”
- transcript helper 负责“把 role/text 组装成 MemPalace transcript”

把这个边界切开之后，责任会更清楚。

## internal split 的收益不只是文件变短

真正的收益是后续继续支持新格式时，改动会更局部：

- 新增一种 chat export
只需要改 `normalize_json`

- 调整 quote transcript heuristic
只需要改 `normalize_transcript`

这会让 review 和回归定位都更直接。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

这些检查通过后，可以确认：

- normalize internal split 没破坏外部 normalization surface
- 新的 facade + parser split 没有打断编译
- 现有 convo mining / normalize CLI / service 回归仍然保持绿色

# 未覆盖项

这次没有继续改：

- `spellcheck.rs`
- `convo_exchange.rs`
- `project_cli_transcript_normalize.rs`
- `miner_convo.rs`

因为目标只是把 normalize internals 按 transcript/json 两层切开，而不是继续扩散到 spellcheck 或 convo mining 其它模块。
