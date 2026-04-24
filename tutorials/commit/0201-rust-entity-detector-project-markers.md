# Commit 0201: Rust entity detector project marker parity

## 背景

完成 person marker 对齐后，继续检查 Python `PROJECT_VERB_PATTERNS`。Rust 之前只覆盖了部分 project marker：

- `building NAME`
- `built NAME`
- `deploy NAME`
- `launch NAME`
- `NAME.py`
- `NAME-core`

Python 还支持更多项目证据，包括 shipping/install/import/version/local，以及多种脚本或配置文件引用。

## 主要目标

- 补齐 Rust 对 Python project marker 的识别。
- 让缺失 marker 可以把候选推进 projects。
- 不改变 person gate、候选抽取、stopwords 或整体分类结构。

## 改动概览

- 更新 `rust/src/entity_detector_score.rs`。
- 新增 `is_project_marker()`，集中判断 project marker。
- 新增 `has_code_reference()`，覆盖 `.py/.js/.ts/.yaml/.yml/.json/.sh`。
- 补齐 `ship/shipping/shipped`、`install/installing/installed`、`import NAME`、`pip install NAME`、`NAME v...`、`NAME-local`。
- 更新 `rust/src/entity_detector.rs`。
- 新增 `entity_detector_detects_project_install_and_import_markers_like_python`。
- 新增 `entity_detector_detects_project_version_and_local_markers_like_python`。

## 关键知识

Project marker 是项目分类的核心证据。候选抽取只说明某个大写词出现了足够次数，真正进入 projects 还需要 project score。

本片仍然保持 Rust 现有简单评分模型：每条命中的 project marker 行加分。Python 的具体权重更细，例如 code ref / versioned 权重更高；权重完全对齐应另开切片，避免把 marker coverage 和分类阈值调整混在一起。

## 补充知识

这片选择的测试都让候选名 `Atlas` 出现三次，避免把 project marker 行为和候选频次规则混在一起。频次规则已经在前面的 entity detector 测试里固定。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_detects_project_install_and_import_markers_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_detects_project_version_and_local_markers_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 candidate regex、stopwords、person scoring、onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面。
