# Commit 0205: Rust entity detector project marker weights

## 背景

0201 补齐了 Rust 对 Python project marker 的覆盖面，0203 又补上了 project ratio gate。继续检查后发现 Rust 的 project marker 权重仍偏低。

Python `score_entity()` 中：

- project verb 每次命中加 2 分。
- versioned / hyphenated marker 加 3 分。
- code reference 加 3 分。

Rust 此前仍是“每行有 project marker 就加 1 分”。在混入少量 person signal 时，这会导致 Rust 因 project ratio 不足而拒绝 Python 会确认的 project。

## 主要目标

- 将 Rust project marker scoring 从布尔判断升级为权重函数。
- project verb 类 marker 加 2 分。
- versioned / hyphenated marker 加 3 分。
- code reference marker 加 3 分。
- 固定 code ref 强证据可以压过少量 person action 的行为。

## 改动概览

- 更新 `rust/src/entity_detector_score.rs`。
- 用 `project_marker_score()` 替代 `is_project_marker()`。
- `has_code_reference()` 命中时加 3 分。
- `NAME-...` hyphenated/versioned marker 命中时加 3 分。
- project verb / install / import / local 等 marker 命中时加 2 分。
- 更新 `rust/src/entity_detector.rs`。
- 新增 `entity_detector_accepts_code_ref_project_weight_like_python`。

## 关键知识

这一片解决的是“marker 已识别但分数不够”的漏检。候选名仍必须满足频次要求，且 project 仍必须通过 0203 的 ratio gate。

测试故意加入 `Atlas said ...` 作为少量 person action：旧 Rust 会得到较低 project score，`person_ratio` 偏高；对齐 code ref 权重后，Rust 和 Python 一样确认 project。

## 补充知识

Rust 目前仍保留早期的 `PROJECT_HINTS` 泛化逻辑，因此这片不是完整的 Python scoring 等价重写。后续如果继续收口，应单独审 `PROJECT_HINTS` 与 Python `PROJECT_VERB_PATTERNS` 的重叠和扩展差异。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_accepts_code_ref_project_weight_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_detects_project_version_and_local_markers_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_does_not_accept_mixed_ratio_project_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未重写 `PROJECT_HINTS`。
- 未改变 candidate regex、stopwords、person scoring、project ratio、onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面.
