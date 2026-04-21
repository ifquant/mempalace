# Commit 0203: Rust entity detector project ratio parity

## 背景

继续对齐 Python `classify_entity()` 时，发现 Rust project 确认条件仍偏宽。

Python 会先计算：

- `total = person_score + project_score`
- `person_ratio = person_score / total`

只有当 `person_ratio <= 0.3` 时，Python 才会把候选确认成 project。Rust 此前只要 person gate 没通过且 `project_score >= 2`，就会进入 projects。这样混合信号候选可能被 Rust 误写成 project，而 Python 会降级为 uncertain。

## 主要目标

- 给 Rust project classification 增加 Python 的 ratio gate。
- 混合 person/project 信号不再因为 project_score 达标就进入 projects。
- 保留已有 project-only 正向检测能力。

## 改动概览

- 更新 `rust/src/entity_detector.rs`。
- 新增 `has_project_ratio`，用整数比较表达 `person_ratio <= 0.3`。
- project 只有在 `project_score >= 2 && has_project_ratio` 时才进入 projects。
- 新增 `entity_detector_does_not_accept_mixed_ratio_project_like_python`。

## 关键知识

Python 的 project gate 看的是 person ratio，而不是单纯比较 project_score 是否存在。这个设计能避免“名字像人、同时也出现 repo/architecture/version 等项目词”的候选被自动归入 project。

Rust 使用 `person_score * 10 <= total_score * 3`，避免引入浮点比较，同时保持 `<= 0.3` 的语义。

## 补充知识

本片只收 project acceptance gate，不重写 score 权重。Rust 与 Python 的 scoring 权重仍有差异，例如 Python 对 code ref/versioned 单独加 3 分；这类权重对齐应另开切片。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_does_not_accept_mixed_ratio_project_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_extracts_multi_word_projects_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_detects_project_install_and_import_markers_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 candidate regex、stopwords、person scoring、project scoring 权重、onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面。
