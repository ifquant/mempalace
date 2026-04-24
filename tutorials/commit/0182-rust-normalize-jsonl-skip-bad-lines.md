# Commit 0182: Rust normalize JSONL bad-line parity

## 背景

上一轮把 Rust normalize 的文件读取改成 lossy UTF-8，对齐 Python `errors="replace"`。继续对照 Python `normalize.py` 的 JSONL parser 时，又发现一个明确的容错差异。

Python 在 `_try_claude_code_jsonl` 和 `_try_codex_jsonl` 中逐行解析 JSONL；如果某一行 `json.loads` 失败，会 `continue` 跳过该行。Rust 此前在同类 parser 中使用 `serde_json::from_str(line).ok()?`，这会让任意一行坏 JSON 直接让整个 parser 返回 `None`。

## 主要目标

- 让 Rust Claude Code JSONL parser 遇到 malformed line 时跳过该行。
- 让 Rust Codex JSONL parser 遇到 malformed line 时跳过该行。
- 保持消息抽取、role 映射、spellcheck 和 session_meta 要求不变。
- 用测试覆盖坏行夹在有效 user/assistant 消息之间仍能 normalize 成 transcript。

## 改动概览

- 更新 `rust/src/normalize_json_jsonl.rs`。
- `try_claude_code_jsonl` 中 `serde_json::from_str` 失败时 `continue`。
- `try_codex_jsonl` 中 `serde_json::from_str` 失败时 `continue`。
- 新增 `claude_code_jsonl_skips_malformed_lines_like_python` 单元测试。
- 新增 `codex_jsonl_skips_malformed_lines_like_python` 单元测试。

## 关键知识

JSONL 文件的一个重要现实特征是“每行独立”。真实导出或日志拼接中，单行损坏不应该阻断整个文件。Python 的实现已经体现了这个策略：坏行跳过，有效行继续参与 messages 收集。

Rust 的 `ok()?` 在 `Option` 返回函数里很方便，但语义更强：它把单行解析失败升级成整个 parser 失败。本次把它改成 `let Ok(entry) = ... else { continue; };`，让行为回到 Python 的逐行容错。

## 补充知识

Codex JSONL 仍然要求存在 `session_meta`，这个规则没有改变。也就是说，跳过坏行只影响“局部损坏容忍”，不会把任意 JSONL 文件误判成 Codex transcript。

测试里保留 spellcheck 期望，例如 `knoe` -> `know`，确保坏行跳过后仍走完整 transcript conversion 流程。

## 验证

- `cargo fmt --check`
- `cargo test normalize_json::jsonl::tests::claude_code_jsonl_skips_malformed_lines_like_python -- --exact`
- `cargo test normalize_json::jsonl::tests::codex_jsonl_skips_malformed_lines_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未调整 JSON object/export parser。
- 未修改 README 或 parity ledger。
- 未改变 Python 实现。
