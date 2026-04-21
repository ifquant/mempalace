# Commit 0188: Rust normalize Codex JSONL payload skip parity

## 背景

继续对照 Python `normalize.py` 和 Rust JSONL normalize 时，发现 Codex JSONL parser 还有一个坏 entry 容错差异。

Python `_try_codex_jsonl` 对每一行都会独立判断：如果 `payload` 不是 dict，直接 `continue`，后续有效消息仍然会参与 transcript 生成。

Rust 此前在 `event_msg` 分支里用 `entry.get("payload")?`。这会在某一行缺 `payload` 时让整个 parser 返回 `None`，丢掉前后已经能正常解析的消息。

## 主要目标

- 让 Rust Codex JSONL parser 对缺失或非对象 `payload` 的 `event_msg` 逐行跳过。
- 保持 malformed JSON line skip、`session_meta` gate 和 user/agent message 提取规则不变。
- 用单元测试固定这个 Python parity 行为。

## 改动概览

- 更新 `rust/src/normalize_json_jsonl.rs`。
- 将缺失 `payload` 的 `event_msg` 从 parser-level abort 改为 line-level skip。
- 新增 `codex_jsonl_skips_event_msg_without_payload_like_python` 测试。

## 关键知识

JSONL normalize 的容错粒度应该是“行”，不是“整个文件”。Codex session 里可能混有不完整、扩展或未来版本 entry；只要 canonical `event_msg` user/agent 消息仍然足够，就应该继续生成 transcript。

`?` 在 parser 函数里很容易把局部坏数据升级成整体失败。本次改成 `let Some(...) else { continue; }`，语义更接近 Python 的逐项过滤。

## 补充知识

这里仍然保留 `has_session_meta` 要求。也就是说，只有确认是 Codex session 的 JSONL，Rust 才会返回 Codex transcript；坏 `event_msg` 不会绕过这个格式识别 gate。

## 验证

- `cargo fmt --check`
- `cargo test normalize_json::jsonl::tests::codex_jsonl_skips_event_msg_without_payload_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 Claude Code JSONL、Claude.ai、ChatGPT、Slack parser。
