# Commit 0194: Rust entity detector scan extension parity

## 背景

继续审 `entity_detector` 的 Python/Rust 行为差异时，发现 scan 文件类型还有一处会影响自动检测结果。

Python `entity_detector.py` 的 prose 文件只包括：

```python
PROSE_EXTENSIONS = {".txt", ".md", ".rst", ".csv"}
```

Readable fallback 则包括 `.py/.js/.ts/.json/.yaml/.yml/.toml/.sh/.rb/.go/.rs` 等。

Rust 此前把 `.json/.jsonl` 也放进 prose extensions。这样在目录里已经有足够 prose 文件时，Rust 仍可能优先读 JSON/JSONL export，而 Python 不会；这会把结构化导出里的 capitalized token 带入 entity detection。

## 主要目标

- 让 Rust entity detector 的 prose/readable extension 列表对齐 Python。
- `.json` 只作为 readable fallback，不作为 prose 文件。
- 移除 `.jsonl` 的 entity-detector scan 支持，因为 Python readable/prose 列表都没有它。

## 改动概览

- 更新 `rust/src/entity_detector_scan.rs`。
- `PROSE_EXTENSIONS` 收窄为 `.txt/.md/.rst/.csv`。
- `READABLE_EXTENSIONS` 扩展为 Python 对应列表：`.py/.js/.ts/.json/.yaml/.yml/.toml/.sh/.rb/.go/.rs` 等。
- 更新 `rust/src/entity_detector.rs` 测试，新增 `scan_for_detection_treats_json_as_readable_fallback_like_python`。

## 关键知识

entity detection 的扫描对象要保守。Python 注释已经明确：code files 会产生太多 capitalized false positives，所以优先 prose，只在 prose 不足时回退 readable。

JSON/JSONL export 里也有类似问题：字段名、角色名、工具名和模型名都可能像实体。Rust 如果把 JSON 当 prose，会让 onboarding/init 自动实体结果偏离 Python。

## 补充知识

这次只改 scan 文件类型分类，不改 entity scoring、stopwords、多词实体、候选频率或 registry 写入逻辑。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::scan_for_detection_treats_json_as_readable_fallback_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 onboarding CLI/MCP schema、registry、split、normalize 或 maintenance 能力面。
