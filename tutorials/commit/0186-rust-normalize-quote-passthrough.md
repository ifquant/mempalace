# Commit 0186: Rust normalize quote-transcript pass-through

## 背景

继续审 `normalize` 的基础入口语义时，发现 Rust 对“已经是 transcript 的文本”处理和 Python 不一致。

Python `normalize.py` 在检测到至少 3 行以 `>` 开头的内容后，直接返回原始 content：

```python
if sum(1 for line in lines if line.strip().startswith(">")) >= 3:
    return content
```

Rust 此前也会识别 quote transcript，但会继续对 user turns 做 spellcheck。这改变了已有 transcript 的内容，不符合 Python 的 pass-through 语义。

## 主要目标

- 让 Rust normalize 对已有 quote transcript 原样 pass-through。
- 保留 JSON/export normalize 中的 user-turn spellcheck。
- 保留独立 spellcheck API，不删除 spellcheck 测试。

## 改动概览

- 更新 `rust/src/normalize.rs`。
- `count_quote_lines(content) >= 3` 时返回 `Some(raw.to_string())`。
- 新增 `normalize_existing_quote_transcript_passes_through_like_python` 测试，确认拼写错误不会被 normalize 修正。
- 删除不再使用的 `normalize_quote_transcript` wrapper。

## 关键知识

normalize 的第一原则是：如果输入已经是 MemPalace transcript，就不要重写它。Python 这里选择了完整 pass-through，而不是“顺便清理”或“顺便 spellcheck”。

Rust 的 spellcheck 对 JSON/export 转 transcript 仍然有价值，但它不应该应用到已经成型的 transcript 文件上，否则用户已有记录会被 normalize 改写。

## 补充知识

本次只影响 quote transcript pass-through。plain text fallback、blank content pass-through、JSON/JSONL parser 和 Slack/ChatGPT/Claude.ai export parser 都不变。

`spellcheck_transcript` 仍然保留在 `spellcheck.rs`，因为它是独立能力并且已有测试覆盖；删除的只是 normalize 层的薄 wrapper。

## 验证

- `cargo fmt --check`
- `cargo test normalize::tests::normalize_existing_quote_transcript_passes_through_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未改变 JSON/export 生成 transcript 时的 spellcheck 行为。
- 未修改 README 或 parity ledger。
- 未改变 Python 实现。
