# Commit 0197: Rust entity detector pronoun proximity parity

## 背景

0196 收紧了 Rust person classification gate：person 必须有足够分数、至少两类 person signal、并满足 person ratio。那一片仍留下一个明确差距：Python 会把名字附近的代词作为 `pronoun` person signal，Rust 还没有。

Python 的逻辑是：找出包含候选名字的行，然后扫描该行前后两行组成的窗口。如果窗口里出现 `she / her / he / him / they / them / their` 等代词，就给 person score 加分，并把 `pronoun` 纳入 signal category。

## 主要目标

- 在 Rust `score_person()` 中加入 pronoun proximity 分数。
- 在 Rust `person_signal_category_count()` 中把 pronoun 作为独立 category。
- 保持 0196 的保守 gate：pronoun 只能作为第二类 signal，不能单独让候选进入 people。

## 改动概览

- 更新 `rust/src/entity_detector_score.rs`。
- 新增 `PRONOUNS` 列表。
- 新增 `pronoun_proximity_hits()`，按 Python 的前后两行窗口统计 pronoun proximity。
- 新增 `contains_pronoun()`，按 ASCII alphabetic 边界拆词，避免子串误判。
- 更新 `rust/src/entity_detector.rs` 测试。
- 新增 `entity_detector_accepts_action_plus_pronoun_person_like_python`。
- 新增 `entity_detector_does_not_accept_pronoun_only_people_like_python`。

## 关键知识

Pronoun proximity 不是“只要附近有代词就确认是人”。Python 的确认条件仍然要求至少两类 signal，所以 pronoun 更适合作为 action/dialogue/addressed 的补充证据。

这个边界很重要：自动 registry 写入宁可少报，也不能因为通用代词出现在附近就把普通词误写成人名。

## 补充知识

Rust 这里用整数计算保留了 0196 的 `person_ratio >= 0.7` 语义，没有引入浮点比较。pronoun 分数加入后，旧的 action-only 和 pronoun-only 测试仍然守住 false positive 边界。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_accepts_action_plus_pronoun_person_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_does_not_accept_pronoun_only_people_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面。
