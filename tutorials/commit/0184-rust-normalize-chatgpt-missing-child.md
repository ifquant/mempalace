# Commit 0184: Rust normalize ChatGPT missing-child parity

## 背景

继续审 `normalize` 的 JSON export parser 时，发现 ChatGPT `mapping` 遍历在缺失 child 节点时有一个 Python/Rust 差异。

Python 在 `_try_chatgpt_json` 里使用 `mapping.get(current_id, {})`。如果当前 child id 不存在，它会得到空 dict，然后没有 message、没有 children，循环自然结束；此前已经收集到的 user/assistant 消息仍然保留，最后只要消息数足够就返回 transcript。

Rust 此前使用 `mapping.get(&current)?`。这意味着只要遍历到一个缺失 child id，整个 parser 就直接返回 `None`，即使前面已经收集到了完整的 user/assistant 对话。

## 主要目标

- 让 Rust ChatGPT JSON normalize 在遇到缺失 child 节点时对齐 Python：停止遍历，而不是丢弃已收集消息。
- 保持 root 查找、visited 防环、children-first traversal 和 message extraction 不变。
- 用单元测试覆盖“有效消息后接 missing child”的情况。

## 改动概览

- 更新 `rust/src/normalize_json_exports.rs` 的 `try_chatgpt_json`。
- 将 `let node = mapping.get(&current)?;` 改成缺失节点时 `break`。
- 新增 `chatgpt_json_keeps_messages_before_missing_child_like_python` 测试。

## 关键知识

ChatGPT export 的 `mapping` 是一棵由 id 连接的树。真实导出或裁剪后的样本里，children 可能引用不存在的节点。Python 的实现把这种情况当作“路径结束”，不会让已解析出的消息失效。

Rust 的 `?` 在这里太强了：它把一个尾部缺失 child 升级成整个 parser 失败。正确行为是 break，因为缺失节点之后没有更多可遍历内容，但之前的 messages 仍然有效。

## 补充知识

本次不改变 Python 的“只跟随第一个 child”的行为。多分支 ChatGPT mapping 仍然按现有语义走主路径，后续如果要支持分支选择，应作为 Rust extension 或单独 parity 决策处理。

## 验证

- `cargo fmt --check`
- `cargo test normalize_json::exports::tests::chatgpt_json_keeps_messages_before_missing_child_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未调整 ChatGPT 多分支 traversal。
- 未修改 Slack、Claude.ai 或 JSONL parser。
- 未修改 README 或 parity ledger。
