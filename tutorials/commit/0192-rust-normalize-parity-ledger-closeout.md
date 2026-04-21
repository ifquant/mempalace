# Commit 0192: Rust normalize parity ledger closeout

## 背景

0181 到 0191 连续完成了 Rust normalize 行为面的 Python parity pass。

这一轮没有再继续写实现代码，而是把已经验证过的 normalize 对齐事实写回 `docs/parity-ledger.md`。这样后续不会继续按“normalize 还有一堆未知缺口”的过时假设推进。

## 主要目标

- 在 parity ledger 的 snapshot 中记录 normalize 行为审计已完成。
- 在 Completed Behavior Audits 中新增 `Transcript normalize`。
- 明确 Rust 的 `normalize` CLI/MCP 入口和 flat JSON message-array parsing 是扩展面，不是 Python 缺口。
- 缩小 Remaining Work 的泛化描述，避免重新打开已完成的 split/normalize 审计。

## 改动概览

- 更新 `docs/parity-ledger.md`。
- 新增 normalize 完成审计条目，覆盖：
  - 500MB file guard
  - lossy UTF-8 reads
  - blank / quote transcript pass-through
  - `>` marker detection
  - Claude Code / Codex JSONL bad-entry tolerance
  - Codex missing payload skip
  - Slack role assignment
  - ChatGPT missing child / empty parts
  - Claude.ai key priority
- 新增本教程文件。

## 关键知识

parity ledger 的价值是防止后续重复盘账。实现切片完成之后，如果不把结论写回 ledger，下一轮 agent 很容易继续把已收口区域当成 `remaining`。

这次选择把 normalize 放进 Completed Behavior Audits，而不是继续在 Remaining Work 里泛泛写“deeper audit”。后续如果发现新的具体差异，应该新增一个明确 gap，而不是重新打开整个 normalize 面。

## 补充知识

Rust 的 `normalize` CLI/MCP 入口本身仍是 `rust superset`，因为 Python 没有同等 public surface。这里完成的是底层 transcript normalize 行为面，而不是把 Rust-only entrypoint 改成 Python surface。

Rust flat JSON message-array parser也保留为扩展面；它不是 Python 缺口，也不是当前要删除的行为。

## 验证

- `git diff -- docs/parity-ledger.md tutorials/commit/0192-rust-normalize-parity-ledger-closeout.md`
- `git status --short`

## 未覆盖项

- 未修改 Rust 运行时代码。
- 未修改 Python 实现。
- 未修改 README。
- 未改变 MCP schema、split、registry、layers 或 maintenance 能力面。
