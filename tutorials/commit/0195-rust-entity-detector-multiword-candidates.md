# Commit 0195: Rust entity detector multi-word candidates

## 背景

继续审 `entity_detector.py` 的候选提取规则时，发现 Python 支持多词 proper noun：

```python
multi = re.findall(r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+)\b", text)
```

例如 `Memory Palace`、`Claude Code` 这种项目名会作为完整 phrase 进入候选池。Rust 此前只提取单个 capitalized word，因此只能看到 `Memory` 和 `Palace`，无法把完整项目名写入 detected projects。

## 主要目标

- 让 Rust entity detector 支持 Python 的多词 proper noun 候选。
- 如果 phrase 中任一 word 是 stopword，则和 Python 一样跳过该 phrase。
- 保持单词候选提取和 3 次频率门槛不变。

## 改动概览

- 更新 `rust/src/entity_detector.rs`。
- 新增多词候选正则：`[A-Z][a-z]+(?:\s+[A-Z][a-z]+)+`。
- 多词 phrase 进入同一个 frequency counts map。
- 新增 `entity_detector_extracts_multi_word_projects_like_python` 测试，固定 `Memory Palace` 可被检测为 project。

## 关键知识

entity detector 的输出会进入 onboarding/init 的 registry bootstrap。单词级候选对人名足够，但对项目名经常不够，因为项目常以双词或多词出现。

如果 Rust 只保留单词候选，会把 `Memory Palace` 拆成两个不准确的实体，或者完全错过完整项目名。Python 已经把这种情况作为候选提取的一部分，所以 Rust 也应该对齐。

## 补充知识

这次只补候选提取，不改 scoring/classification 阈值。多词 phrase 后续仍然需要通过现有 project/person score gate 才会进入输出。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_extracts_multi_word_projects_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 onboarding CLI/MCP schema、registry、split、normalize 或 maintenance 能力面。
