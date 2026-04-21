# Commit 0199: Rust entity detector stopwords parity

## 背景

继续盘 Rust entity detector 的候选抽取差异时，发现 Rust 的 stopwords 表明显小于 Python。

Python `entity_detector.py` 把大量常见代词、UI 动词、技术词、抽象概念和句首 filler 词都排除在候选之外。Rust 之前只覆盖了少量常见词，因此 `Click`、`Memory Palace` 这类在 Python 会被过滤的候选仍可能进入 Rust scoring。

## 主要目标

- 将 Rust stopwords 表扩展到当前 Python 的唯一 stopword 集合。
- 单词候选和 multi-word 候选都沿用同一 stopword 过滤边界。
- 修正旧 multi-word 测试，让正向样例不依赖 Python 已经过滤的 `memory`。

## 改动概览

- 更新 `rust/src/entity_detector_score.rs`。
- 用 Python `STOPWORDS` 的唯一集合替换 Rust 的小 stopword 表。
- 更新 `rust/src/entity_detector.rs`。
- 将 multi-word 正向 fixture 从 `Memory Palace` 改为 `Atlas Core`。
- 新增 `entity_detector_filters_python_stopwords`，覆盖单词 stopword。
- 新增 `entity_detector_filters_multi_word_phrases_with_python_stopwords`，覆盖 multi-word phrase 内任一词为 stopword 的情况。

## 关键知识

stopwords 是 false positive 控制面，不是展示层细节。候选一旦进入 scoring，就可能经过 person/project gate 写进 registry，所以 Rust 必须尽量复用 Python 的过滤边界。

旧的 `Memory Palace` 正向测试名义上说“like Python”，但当前 Python 的 `memory` 已是 stopword，因此它实际不再代表 Python 行为。本片把正向 multi-word 能力保留在 `Atlas Core`，并单独加负向测试固定 `Memory Palace` 被过滤。

## 补充知识

Python 源表里有重复项，例如 `then`、`true`、`false`、`none`、`get`、`copy`、`find`、`system`。Rust 使用唯一集合即可，因为 `is_stopword()` 只关心成员关系。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_extracts_multi_word_projects_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_filters_python_stopwords -- --exact`
- `cargo test entity_detector::tests::entity_detector_filters_multi_word_phrases_with_python_stopwords -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 candidate regex、person scoring、project scoring、onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面。
