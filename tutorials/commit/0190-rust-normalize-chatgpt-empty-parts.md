# Commit 0190: Rust normalize ChatGPT empty parts parity

## 背景

继续对照 Python `normalize.py` 的 ChatGPT parser 时，发现 `content.parts` 拼接规则还有一个输出级差异。

Python 对 parts 的处理是：

```python
" ".join(str(p) for p in parts if isinstance(p, str) and p).strip()
```

也就是说，空字符串 part 会被过滤掉。Rust 此前把所有 string part 都纳入 join，遇到 `["Ship", "", "today"]` 会生成 `"Ship  today"`，多出一个空格。

## 主要目标

- 让 Rust ChatGPT parser 过滤空字符串 part。
- 保持非空 string part 的顺序和空格拼接方式不变。
- 用测试固定 `["Ship", "", "today"]` 这类输入的 Python parity 输出。

## 改动概览

- 更新 `rust/src/normalize_json_exports.rs`。
- 在 ChatGPT `content.parts` 提取时过滤 `text.is_empty()`。
- 新增 `chatgpt_json_ignores_empty_parts_like_python` 测试。

## 关键知识

ChatGPT export 的 `parts` 可能包含空字符串。这里的 parity 重点不是“能不能解析”，而是 transcript 文本是否稳定一致。

空 part 如果不提前过滤，会在 join 后留下额外空格；这会影响 snapshot、搜索命中显示、后续 mining 结果和对齐判断。

## 补充知识

这次只改 ChatGPT mapping parser 的 `content.parts` join 规则。Claude.ai、Slack、Codex JSONL、Claude Code JSONL 的 content extraction 不变。

## 验证

- `cargo fmt --check`
- `cargo test normalize_json::exports::tests::chatgpt_json_ignores_empty_parts_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 MCP schema、split、registry、layers 或 maintenance 能力面。
