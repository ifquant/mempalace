# 背景

`rust/src/normalize_json.rs` 之前把两类完全不同的输入路径都放在一个文件里：

- JSONL 聊天导出
- 普通 JSON 聊天导出

虽然它们都属于“chat export normalization”，但真实格式差异很大：

- Claude Code / Codex JSONL 更像事件流
- ChatGPT / Claude / Slack JSON 更像树形或数组结构

继续把这些分支全塞在一个文件里，会让后续每新增一种导出格式时都继续往同一个文件堆逻辑。

# 主要目标

这次提交的目标是把 `normalize_json.rs` 继续拆成：

- JSONL 一层
- JSON export 一层
- facade 一层

同时保持外部 `normalize_json::try_normalize_json()` surface 不变。

# 改动概览

这次新增了两个内部模块：

- `rust/src/normalize_json_jsonl.rs`
- `rust/src/normalize_json_exports.rs`

拆分后的职责边界是：

## `normalize_json_jsonl`

负责：

- `try_claude_code_jsonl()`
- `try_codex_jsonl()`

这层只关心逐行 JSONL 的事件流导出。

## `normalize_json_exports`

负责：

- `try_flat_messages_json()`
- `try_claude_ai_json()`
- `try_chatgpt_json()`
- `try_slack_json()`
- `extract_content()`

这层只关心普通 JSON 导出和共享内容提取逻辑。

## `normalize_json`

现在只保留：

- `try_normalize_json()`
- 子模块声明
- 顶层路由

也就是外部统一入口。

# 关键知识

## 1. JSONL 和 JSON 虽然都叫 JSON，但解析思路并不一样

工程上最容易混淆的一点是：它们都叫 JSON，所以会被习惯性放在一起。

但实际上：

- JSONL 是“每行一个独立 JSON 事件”
- JSON export 更常见的是“整个文件就是一个对象或数组”

这两类输入的遍历方式、错误面、分支组织方式都不一样，拆开后更容易维护。

## 2. facade 统一入口能保住上层调用不变

这次没有让上层去分别调用：

- `try_claude_code_jsonl()`
- `try_chatgpt_json()`

而是继续只保留：

- `try_normalize_json()`

这样做的好处是，上层永远只认“给我一段内容，我来试着识别是哪种 chat export”，内部格式支持则可以继续演化。

# 补充知识

## 1. 共享 `extract_content()` 单独集中，比在每个 parser 里各写一套更稳

聊天导出里最烦的部分之一，不是 role 判断，而是 content 结构经常不同：

- string
- text object
- array of parts

把 `extract_content()` 收到 `normalize_json_exports` 里之后，至少这类“内容抽取”逻辑就不会在各个 parser 分支里反复复制。

## 2. 这种拆法适合后续继续扩格式支持

如果以后再补一种：

- 新的 JSONL 事件流导出
- 新的普通 JSON 导出

维护者会更自然地知道应该把它放去哪里，而不是继续把所有 parser 都堆回 `normalize_json.rs`。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

这次没有改这些内容：

- 没有改变 normalization 的外部行为
- 没有改变 transcript spellcheck 链路
- 没有增加新的 chat export 格式支持
- 没有继续拆 `normalize_json_exports` 内部的单个格式 parser
- 没有改 Python `normalize.py`
