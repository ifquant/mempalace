# Commit 0198: Rust entity detector candidate length parity

## 背景

继续对齐 Python `entity_detector.py` 时，发现 Rust 单词候选 regex 仍有一个边界差异。

Python 使用 `\b([A-Z][a-z]{1,19})\b` 抽取单词候选。这意味着：

- 两个字母的名字可以成为候选，例如 `Jo`。
- 单词候选最长 20 个字母。

Rust 此前使用 `\b[A-Z][a-z]{2,}\b`，会漏掉两个字母的名字，也会接受超长单词。

## 主要目标

- 让 Rust 单词候选长度边界与 Python 一致。
- 两字母候选能继续走 person/project scoring。
- 超过 Python 上限的单词不进入单词候选。

## 改动概览

- 更新 `rust/src/entity_detector.rs`。
- 将单词候选 regex 从 `{2,}` 改为 `{1,19}`。
- 新增 `entity_detector_accepts_two_letter_names_like_python`。
- 新增 `entity_detector_ignores_overlong_single_word_candidates_like_python`。

## 关键知识

候选抽取边界会直接影响后续 scoring。这里不是简单的正则细节：漏掉两个字母名字会让 Rust 少写真实 person；接受超长单词则会扩大 false positive 面。

Python 的 20 字母上限只作用于单词候选。多词 proper noun 仍走另一条 multi-word regex，本片不改那条路径。

## 补充知识

测试里的超长样例必须真正超过 20 个字母。`Supercalifragilistic` 正好是 20 个字母，仍符合 Python 上限；因此测试使用更长的 `Supercalifragilisticexpialidocious`。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_accepts_two_letter_names_like_python -- --exact`
- `cargo test entity_detector::tests::entity_detector_ignores_overlong_single_word_candidates_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 multi-word candidate regex。
- 未改变 onboarding CLI/MCP schema、registry runtime、split、normalize 或 maintenance 能力面。
