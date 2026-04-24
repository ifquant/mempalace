# Commit 0191: Rust normalize Claude.ai key priority parity

## 背景

继续对照 Python `normalize.py` 的 Claude.ai export parser 时，发现顶层 dict 的 key 优先级有一个行为差异。

Python 使用：

```python
data = data.get("messages", data.get("chat_messages", []))
```

这意味着只要 `messages` key 存在，Python 就会使用它；即使 `messages` 不是 list，也不会再回退到 `chat_messages`。

Rust 此前更宽松：如果 `messages` 不是数组，会继续尝试 `chat_messages`。这会让一些 Python 不会 normalize 的畸形 export 在 Rust 里被 normalize，造成输出面不一致。

## 主要目标

- 让 Rust Claude.ai parser 遵守 Python 的顶层 key 优先级。
- `messages` 存在但不是数组时返回 `None`，不回退 `chat_messages`。
- 保留 `messages` 缺失时使用 `chat_messages` 的行为。

## 改动概览

- 更新 `rust/src/normalize_json_exports.rs`。
- 重写 Claude.ai 顶层 list 选择逻辑，先判断 object，再按 Python 顺序选择 key。
- 新增 `claude_ai_json_does_not_fallback_when_messages_key_is_invalid_like_python` 测试。

## 关键知识

parity 不只是“多支持一点也没坏处”。normalize 是后续 mining/search 的入口，Rust 如果对 Python 不会识别的畸形输入产生 transcript，就会让两条实现线的行为账继续分叉。

这里选择忠实复刻 Python 的 key 优先级，而不是保留 Rust 的宽松 fallback。

## 补充知识

这次只改 Claude.ai 顶层 dict 的 `messages` / `chat_messages` 选择规则。privacy export 数组、flat messages 数组、ChatGPT、Slack、Codex JSONL 和 Claude Code JSONL 都不变。

## 验证

- `cargo fmt --check`
- `cargo test normalize_json::exports::tests::claude_ai_json_does_not_fallback_when_messages_key_is_invalid_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 MCP schema、split、registry、layers 或 maintenance 能力面。
