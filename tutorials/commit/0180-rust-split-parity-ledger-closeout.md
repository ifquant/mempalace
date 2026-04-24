# Commit 0180: Rust split parity ledger closeout

## 背景

从 `0172` 到 `0179`，Rust transcript split 连续完成了一组行为级 Python parity 修正。覆盖范围已经从单个 helper 扩展到完整用户调用路径：

- people 检测从泛化大写词改成 Python fallback known people。
- source stem、subject、最终 filename sanitize 对齐 Python 命名规则。
- transcript 读取改成 lossy UTF-8，匹配 Python `read_text(errors="replace")`。
- CLI 补齐 `--file`、`--source`、`MEMPALACE_SOURCE_DIR` 和默认 `~/Desktop/transcripts`。
- people 检测支持 `~/.mempalace/known_names.json` 的 list/object 配置和 `username_map`。

这些修正已经分别有代码测试和教程。如果 parity ledger 仍只写“继续 deeper audit”，后续 agent 可能会重复审 split 或误以为 split 仍是未盘状态。

## 主要目标

- 在 `docs/parity-ledger.md` 明确记录 transcript split parity pass 已完成。
- 保持 Remaining Work 仍存在，但把范围限定到 split 之外的行为审计。
- 不改 Rust 运行时代码。

## 改动概览

- 在 ledger Snapshot 中新增 split 行为审计完成说明。
- 扩展 Python CLI Surface 中 `split` 的说明，写明 Rust 已覆盖：
  - `--source`
  - `--file`
  - `MEMPALACE_SOURCE_DIR`
  - lossy text reads
  - Python-style naming
  - fallback people detection
  - `known_names.json`
- 新增 `Completed Behavior Audits` 小节。
- 在该小节中把 `Transcript mega-file split` 标为 `aligned`。
- 调整 Remaining Work 的 `Deeper non-CLI behavior audit` 说明，明确后续审计应聚焦 split 之外。

## 关键知识

ledger 的作用不是只记录缺口，也要记录已经完成的行为审计。否则“remaining” 会变成永远打开的模糊任务，后续实现容易重复做已经验证过的路径。

本次仍保留 `Deeper non-CLI behavior audit` 为 `remaining`，因为整个 Rust/Python 行为面还没有全部逐项盘完；只是 transcript split 这一组不能再被泛化地当作未完成项。

## 补充知识

这一类 closeout commit 不需要新增 Rust 测试，因为它不改变运行时代码。验证重点是检查 ledger 结论和最近 split 实现提交是否一致。

后续如果发现 split 新缺口，应该新增具体条目，例如“split X edge case”，而不是重新打开笼统的 transcript split audit。

## 验证

- `rg -n "split|Completed Behavior Audits|Remaining Work" docs/parity-ledger.md`
- `git diff --check`

## 未覆盖项

- 未修改 Rust 代码。
- 未修改 README。
- 未关闭整个 `Deeper non-CLI behavior audit`，只关闭 transcript split 这一组。
