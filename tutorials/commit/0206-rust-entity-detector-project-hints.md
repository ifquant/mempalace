# Commit 0206: Rust entity detector project hint parity

## 背景

继续收口 project scoring 时，发现 Rust 还保留了一个 Python 没有的泛化扩展：`PROJECT_HINTS`。

Rust 会把 `Atlas architecture`、`Atlas repo`、`Atlas system` 这类裸短语当作 project signal。Python `PROJECT_VERB_PATTERNS` 只接受更明确的 `the Atlas architecture`、`the Atlas pipeline`、`the Atlas system`、`the Atlas repo`。

这个差异会让 Rust 把 Python 会降级为 uncertain 的候选误写成 projects。

## 主要目标

- 移除 Rust 的泛化 `PROJECT_HINTS` 扩展面。
- 补上 Python 精确支持的 `the NAME architecture/pipeline/system/repo` project marker。
- 固定裸 `NAME architecture/repo/system` 不进入 projects。

## 改动概览

- 更新 `rust/src/entity_detector_score.rs`。
- 删除 `PROJECT_HINTS` 常量和 `score_project()` 中的泛化 hint 扫描。
- 在 `project_marker_score()` 中加入 `the NAME architecture`、`the NAME pipeline`、`the NAME system`、`the NAME repo`。
- 将 `score_project()` 的未使用 `text` 参数改为 `_text`，保留调用签名。
- 更新 `rust/src/entity_detector.rs`。
- 新增 `entity_detector_accepts_the_project_context_markers_like_python`。
- 新增 `entity_detector_rejects_bare_project_hints_like_python`。

## 关键知识

这片是收窄 Rust superset，而不是新增能力。对 entity detector 来说，false positive 比漏掉一个候选更危险，因为自动检测结果会进入 registry/onboarding 体系。

Python 的 project context marker 有一个重要限定词 `the`。这个限定减少了普通短语或标题片段被误当项目名的概率。

## 补充知识

`score_project()` 仍保留 `_text` 参数，是为了不扩大本片到调用层重构。后续如果要进一步整理 scoring API，可以单独切片移除该参数。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_accepts_the_project_context_markers_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_rejects_bare_project_hints_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_extracts_multi_word_projects_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 candidate regex、stopwords、person scoring、project ratio、onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面.
