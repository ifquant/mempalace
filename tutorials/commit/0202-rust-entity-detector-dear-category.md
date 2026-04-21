# Commit 0202: Rust entity detector dear category parity

## 背景

复查 0200 的 person marker parity 时，发现 `dear NAME` 的分类归属写错了。

Python `entity_detector.py` 把 `dear NAME` 放在 `PERSON_VERB_PATTERNS`，因此它在 `classify_entity()` 中属于 action category。Python 的 direct-address regex 只覆盖 `hey NAME`、`thank(s) NAME`、`hi NAME`，不覆盖 `dear NAME`。

这意味着 `Dear Avery + Avery said + Avery wrote` 在 Python 里只有 action category，会降级为 uncertain；Rust 0200 错把 `dear NAME` 当 addressed，导致 action + addressed 两类信号成立，可能误写入 people。

## 主要目标

- 将 Rust 的 `dear NAME` 从 addressed category 移出。
- 让 `dear NAME` 作为 action signal 参与 person score。
- 固定 action-only dear 样例不能进入 people。

## 改动概览

- 更新 `rust/src/entity_detector_score.rs`。
- `score_person()` 对 `dear NAME` 加 action 分。
- `person_signal_category_count()` 将 `dear NAME` 计入 action category。
- `is_direct_address()` 不再匹配 `dear NAME`。
- 更新 `rust/src/entity_detector.rs`。
- 将 `entity_detector_accepts_dear_direct_address_like_python` 改为 `entity_detector_does_not_accept_dear_action_only_people_like_python`。
- 更新 `tutorials/commit/0200-rust-entity-detector-person-markers.md`，标明 0200 中 `dear` category 的纠偏。

## 关键知识

这里不是简单的命名问题。0196 之后，person 必须至少有两类 signal category 才能进入 people。如果把 `dear NAME` 错归为 addressed，action-only 文本就会被误判为两类信号。

Python 的行为已用现场检查确认：`Dear Avery`、`Avery said`、`Avery wrote` 的 person score 达标，但 category 只有 action，因此分类为 uncertain。

## 补充知识

这个提交保留了 0200 对 `[NAME]`、`"NAME said`、`> NAME ` 的 dialogue marker 对齐；只纠正 `dear NAME` 的 category。

## 验证

- `python3` 现场调用 `extract_candidates()`、`score_entity()`、`classify_entity()` 确认 Python 对 `Dear Avery` 样例返回 `uncertain`
- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_does_not_accept_dear_action_only_people_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 candidate regex、stopwords、project scoring、onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面。
