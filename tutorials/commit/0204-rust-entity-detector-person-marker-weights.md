# Commit 0204: Rust entity detector person marker weights

## 背景

前几片补齐了 person marker 的覆盖面，但 Rust 仍没有对齐 Python 的 person marker 权重。

Python `score_entity()` 中：

- dialogue marker 每次命中加 3 分。
- direct address 每次命中加 4 分。
- person action 每次命中加 2 分。

Rust 此前已经对 action/pronoun 使用 2 分，但 dialogue 和 direct address 仍然每行只加 1 分。这会导致 `NAME:` + action、`hey NAME` + action 这类 Python 会确认的 person 在 Rust 中低于 `person_score >= 5` gate。

## 主要目标

- 对齐 Rust person marker 权重到 Python。
- dialogue + action 可以达到确认阈值。
- direct address + action 可以达到确认阈值。
- 保留 action-only / dear action-only 的保守边界。

## 改动概览

- 更新 `rust/src/entity_detector_score.rs`。
- `is_dialogue_marker()` 命中时加 3 分。
- `is_direct_address()` 命中时加 4 分。
- 更新 `rust/src/entity_detector.rs`。
- 新增 `entity_detector_accepts_dialogue_plus_action_score_like_python`。
- 新增 `entity_detector_accepts_direct_address_plus_action_score_like_python`。

## 关键知识

0204 不改变 signal category。它只改变同一类 signal 的分数，使 Rust 能通过 0196 的 `person_score >= 5` gate。

这和 0202 的 `dear NAME` 纠偏不冲突：`dear NAME` 仍属于 action，不属于 direct address，因此 `Dear Avery + Avery said + Avery wrote` 仍不会因为两类 signal 成立而进入 people。

## 补充知识

Rust 目前仍按行判断 marker，不统计同一行内多次 direct address 或 dialogue marker。这与 Python 的逐 regex match 计数仍有细节差异，但本片先对齐最影响 gate 的基础权重。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_accepts_dialogue_plus_action_score_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_accepts_direct_address_plus_action_score_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_does_not_accept_dear_action_only_people_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 candidate regex、stopwords、project scoring、project ratio、onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面。
