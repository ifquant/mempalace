# Commit 0183: Rust normalize Slack role parity

## 背景

继续对照 Python `normalize.py` 和 Rust normalize export parser 时，发现 Slack JSON parser 有一个多方对话下的角色分配差异。

Python 用 `seen_users` 字典保存每个 Slack `user_id` 首次分配到的 role。新用户出现时，根据上一条消息的 role 决定它是 `user` 还是 `assistant`；之后同一个用户再次出现，就直接复用字典里的 role。

Rust 此前只保存 seen user 列表，并对已见用户用列表下标奇偶推断 role。这在两人对话里通常没问题，但三人及以上的消息序列中会出错：第三个用户可能首次被分配为 `assistant`，后续再次出现时却因为下标为偶数被推断成 `user`。

## 主要目标

- 让 Rust Slack JSON normalize 的 user role assignment 对齐 Python。
- 保持其他 export parser 不变。
- 用测试覆盖“第三个用户首次分配为 assistant，后续继续保持 assistant”的场景。

## 改动概览

- 更新 `rust/src/normalize_json_exports.rs` 的 `try_slack_json`。
- 将 `seen_users` 从 `Vec<String>` 改为 `BTreeMap<String, &'static str>`。
- 新用户首次出现时按 Python 规则分配 role，并写入 map。
- 已见用户再次出现时直接复用 map 中的 role。
- 新增 `slack_json_preserves_assigned_role_for_returning_third_user` 单元测试。

## 关键知识

Python Slack parser 的目标不是识别真实人类/AI，而是给 Slack 多人消息分配一个稳定的交替结构，方便后续 exchange chunking。这个结构必须对同一个 Slack user_id 稳定，否则同一个人会在 transcript 中一会儿是 user，一会儿是 assistant。

Rust 之前用下标奇偶推断 role，看起来像是在复刻“交替”，但丢失了“首次分配”的状态。正确模型是 `user_id -> assigned_role`，不是 `position -> inferred_role`。

## 补充知识

测试序列使用 A、B、A、C、C：

- A 首次出现为 `user`。
- B 首次出现在 user 后，为 `assistant`。
- A 再次出现仍为 `user`。
- C 首次出现在 user 后，为 `assistant`。
- C 再次出现也必须保持 `assistant`，不能变成 `user`。

## 验证

- `cargo fmt --check`
- `cargo test normalize_json::exports::tests::slack_json_preserves_assigned_role_for_returning_third_user -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未改变 Slack 文本清理或 user_id 选择逻辑。
- 未调整 Claude.ai、ChatGPT 或 JSONL parser。
- 未修改 README 或 parity ledger。
