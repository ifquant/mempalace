# Commit 0200: Rust entity detector person marker parity

## 背景

继续对齐 Python `entity_detector.py` 的 person signal patterns 时，发现 Rust 还少几类 Python 已支持的 person marker：

- `dear NAME` direct address
- `[NAME]` dialogue marker
- `"NAME said` quoted dialogue marker

这些 marker 本身不应该绕过 0196 的保守 person gate，但应当作为 person signal category 参与分类。

## 主要目标

- 补齐 Rust 对 Python person marker 的识别。
- 让新增 marker 同时影响 person score 和 signal category。
- 保持 person gate 不变：仍需要分数、两类 signal、person ratio。

## 改动概览

- 更新 `rust/src/entity_detector_score.rs`。
- 新增 `is_dialogue_marker()`，覆盖 `NAME:`、`> NAME:`、`> NAME `、`[NAME]`、`"NAME said`。
- 新增 `is_direct_address()`，覆盖 `hey NAME`、`thank(s) NAME`、`hi NAME`、`dear NAME`。
- 更新 `rust/src/entity_detector.rs`。
- 新增 `entity_detector_accepts_dear_direct_address_like_python`。
- 新增 `entity_detector_accepts_bracket_dialogue_marker_like_python`。
- 新增 `entity_detector_accepts_quoted_said_dialogue_marker_like_python`。

## 关键知识

Person marker 是证据来源，不是直接确认。Rust 的分类仍然沿用前面几片建立的 gate：action-only、pronoun-only、ratio 不足都不能自动进 people。

这片只补 marker coverage，不调整 Python/Rust 的具体权重差异。权重对齐会影响更多 fixture，应该单独切片。

## 补充知识

Python 的 dialogue pattern 里 `^>\s*{name}[:\s]` 同时支持 `> Name:` 和 `> Name ...`。Rust 以前只支持 `> name:`，本片增加 `> name ` 覆盖这个边界。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_accepts_dear_direct_address_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_accepts_bracket_dialogue_marker_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_accepts_quoted_said_dialogue_marker_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 candidate regex、stopwords、project scoring、onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面。
