# Commit 0181: Rust normalize lossy-read parity

## 背景

split 行为审计已经关闭后，下一轮转向 transcript-prep 的另一个入口：`normalize`。对照 Python `normalize.py` 时，发现文件读取容错存在同类差异。

Python normalize 使用：

```python
open(filepath, "r", encoding="utf-8", errors="replace")
```

这意味着真实导出的聊天文件即使包含少量无效 UTF-8 字节，也会用 replacement character 继续处理。Rust 此前在 `normalize_conversation_file` 里使用严格 `String::from_utf8`，遇到坏字节会返回 `None`，CLI 会把它当成 unsupported/unreadable 文件。

## 主要目标

- 让 Rust normalize 文件读取对齐 Python 的 `errors="replace"` 行为。
- 保持 normalize parser、spellcheck、JSON 格式识别逻辑不变。
- 用文件级测试覆盖无效 UTF-8 字节不会导致 normalize 失败。

## 改动概览

- 更新 `rust/src/normalize.rs` 的 `normalize_conversation_file`。
- 文件仍通过 `fs::read` 读取 bytes。
- bytes 转字符串从严格 `String::from_utf8` 改为 `String::from_utf8_lossy(...).into_owned()`。
- 新增 `normalize_file_tolerates_invalid_utf8_like_python` 测试。
- 测试构造包含 `0xff` 坏字节的 plain transcript，并验证 normalize 结果保留前后文本和 Unicode replacement character。

## 关键知识

Rust 的 `String::from_utf8` 是严格解码，只要出现一个非法字节就失败。Python 的 `errors="replace"` 更适合处理真实导出的 transcript，因为文件可能来自终端、浏览器、复制粘贴或第三方导出工具。

这里用 `String::from_utf8_lossy` 是 Rust 中最接近 Python `errors="replace"` 的方式：有效 UTF-8 原样保留，无效字节替换为 `�`。

## 补充知识

本次只修文件读取层，不改变 `normalize_conversation` 对已传入字符串的处理。后续如果继续审 JSONL parser，还要单独确认“某一行 JSON 解析失败时是否跳过而不是整体失败”等行为。

这个切片和之前 split lossy-read parity 是同一类输入容错对齐，但作用面不同：split 处理 mega-file 拆分，normalize 处理单个聊天导出/转写文件。

## 验证

- `cargo fmt --check`
- `cargo test normalize::tests::normalize_file_tolerates_invalid_utf8_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未调整 JSONL parser 对单行损坏 JSON 的容错行为。
- 未修改 README 或 parity ledger。
- 未改变 Python 实现。
