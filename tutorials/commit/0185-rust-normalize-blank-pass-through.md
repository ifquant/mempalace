# Commit 0185: Rust normalize blank pass-through

## 背景

继续审 `normalize` 基础入口语义时，发现 Python 和 Rust 对空白文件的处理不同。

Python `normalize.py` 在读取文件后先判断：

```python
if not content.strip():
    return content
```

也就是说，空文件或只包含空白字符的文件会原样返回。Rust 此前在 `normalize_conversation` 中把 `raw.trim().is_empty()` 当作 `None`，CLI 会把它渲染为 unsupported/unreadable normalize error。

## 主要目标

- 让 Rust normalize 对空白内容的行为对齐 Python：原样 pass-through。
- 保持 JSON parser、quote transcript parser 和 spellcheck 行为不变。
- 用单元测试锁住空白内容不会变成 unsupported。

## 改动概览

- 更新 `rust/src/normalize.rs`。
- `normalize_conversation` 遇到 `content.is_empty()` 时返回 `Some(raw.to_string())`。
- 新增 `normalize_blank_content_passes_through_like_python` 测试。

## 关键知识

normalize 的职责是“如果是已知 chat export，就转成 transcript；否则 plain text pass-through”。空白文件虽然没有可 normalize 的对话，但它仍然是一个可读 plain text 输入。Python 因此选择返回原内容，而不是把它当成错误。

Rust 之前返回 `None` 的语义太强，会让上层 CLI 把空白文件当成不支持格式。这和 Python 的 pass-through model 不一致。

## 补充知识

本次只改变 `normalize_conversation` 的 core 行为，因此 CLI JSON/human 输出会自然跟随：空白文件现在会产生 `kind=normalize` summary，而不是 error payload。

后续如果需要更精细的 CLI 用户提示，可以单独做 UI/输出层改动；本次只对齐 normalize 语义。

## 验证

- `cargo fmt --check`
- `cargo test normalize::tests::normalize_blank_content_passes_through_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未调整 quote transcript pass-through/spellcheck 差异。
- 未修改 README 或 parity ledger。
- 未改变 Python 实现。
