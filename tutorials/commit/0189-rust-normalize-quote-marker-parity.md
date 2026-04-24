# Commit 0189: Rust normalize quote marker parity

## 背景

继续对照 Python normalize 的基础 pass-through 规则时，发现已有 transcript 的 quote marker 判断仍有一个细节差异。

Python 使用：

```python
line.strip().startswith(">")
```

Rust 此前只统计 `"> "`，也就是必须有一个空格。这会导致 `>user text` 这种没有空格但仍以 `>` 开头的旧 transcript 不被识别为已有 transcript，从而进入后续 normalize 路径。

## 主要目标

- 让 Rust quote transcript 检测与 Python 的 `startswith(">")` 一致。
- 保持已有 `> user text` 行为不变。
- 用测试固定无空格 quote marker 的 pass-through 行为。

## 改动概览

- 更新 `rust/src/normalize_transcript.rs`。
- `count_quote_lines` 从检测 `"> "` 改为检测 `'>'`。
- 更新 `rust/src/normalize.rs`，新增 `normalize_quote_markers_without_space_count_like_python` 测试。

## 关键知识

已有 transcript 的判断应该尽量宽松。normalize 的职责不是重排用户已经保存好的 transcript，而是把聊天 export 转换成 transcript。

Python 对 `>` marker 没有要求后面必须跟空格，因此 Rust 也不应该额外收紧格式，否则会把旧数据或手写 transcript 误判为普通文本。

## 补充知识

这次只影响“是否已有 transcript”的计数逻辑。实际生成 transcript 时，Rust 仍然通过 `messages_to_transcript` 输出 `> text` 的带空格格式。

## 验证

- `cargo fmt --check`
- `cargo test normalize::tests::normalize_quote_markers_without_space_count_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 JSON/JSONL export parser、MCP schema、split、registry、layers 或 maintenance 能力面。
