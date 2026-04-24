# Commit 0196: Rust entity detector person gate parity

## 背景

继续审 `entity_detector.py` 的分类规则时，发现 Rust 对 person 的确认过宽。

Python 不会因为同一种 person action 重复很多次就直接确认 person。它要求：

- person ratio 至少 0.7
- person score 至少 5
- person signals 至少来自两类 category，例如 action + addressed / dialogue / pronoun

Rust 此前只要 `person_score >= 2` 且不低于 project score，就会把候选写入 people。这样 `Jordan said/wrote/pushed...` 这种只有 action category 的文本，会被 Rust 自动写入 registry，但 Python 会降级为 uncertain。

## 主要目标

- 收紧 Rust person classification gate，接近 Python 的核心确认条件。
- action-only person signal 不再自动进入 people。
- 更新正向测试数据，让它们显式包含第二类 person signal，而不是依赖旧宽松阈值。

## 改动概览

- 更新 `rust/src/entity_detector_score.rs`。
- 新增 `person_signal_category_count()`，统计 action / dialogue / addressed 三类 person signal。
- 更新 `rust/src/entity_detector.rs`。
- person 只有在 `person_score >= 5`、signal category 数量至少为 2、且 person ratio 至少 0.7 时才进入 people。
- 新增 `entity_detector_does_not_accept_action_only_people_like_python` 测试。
- 新增 `entity_detector_requires_python_person_ratio` 测试。
- 更新 bootstrap、CLI、MCP、service 测试 fixtures，为正向自动检测样例加入 `hey NAME` direct-address signal。

## 关键知识

entity detection 的确认规则必须偏保守。自动写入 registry 的 false positive 会污染后续 lookup、onboarding 和 mining taxonomy。

Python 的规则专门防止“同一种句式重复出现”被误判为高置信 person。Rust 这次对齐的是这个保守边界，而不是追求更多自动检测命中。

## 补充知识

Rust 当前没有完整实现 Python 的 pronoun proximity signal。本次只统计 Rust 已经能识别和打分的 action、dialogue、addressed 三类。后续如果补 pronoun proximity，应单独做一片。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_does_not_accept_action_only_people_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_requires_python_person_ratio -- --exact`
- `cargo test entity_detector::tests::entity_detector_prefers_prose_files_and_detects_people_projects -- --exact`
- `cargo test bootstrap::tests::bootstrap_detects_rooms_and_entities_and_writes_files -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面。
