# Commit 0179: Rust split known-names config parity

## 背景

Rust split 已经连续对齐了默认 people 检测、source stem、subject、最终 filename sanitize、lossy 读取、`--file`、`--source` 和默认 source 目录。继续对照 Python `split_mega_files.py` 时，剩下一个明确的行为差异：Python 会读取 `~/.mempalace/known_names.json` 来扩展人名检测。

Rust 此前只使用 fallback known people 名单，无法识别用户配置的人名，也无法通过 transcript 里的 `/Users/<username>/` 工作目录提示映射到真实姓名。

## 主要目标

- 让 Rust split 的 people 检测支持 Python 的 `known_names.json` 配置格式。
- 支持 list 和 object 两种配置形态。
- 支持 object 里的 `username_map`。
- 保持 missing/invalid config 回退 fallback known people。

## 改动概览

- 在 `rust/src/split.rs` 增加 `KnownNamesConfig` untagged enum。
- 增加 `load_known_names_config`，从 `~/.mempalace/known_names.json` 读取 JSON；读取或解析失败时返回 `None`。
- 增加 `known_people_and_username_map`，复刻 Python 语义：
  - list 配置直接作为 known people。
  - object 配置读取 `names`，没有 `names` 时使用空名单。
  - object 配置读取 `username_map`，没有时为空 map。
  - 没有有效配置时使用 fallback known people。
- `extract_people` 改为先加载配置，再交给 `extract_people_with_config`。
- `extract_people_with_config` 使用 `BTreeSet` 去重并排序，保持 Python `sorted(found)` 行为。
- 新增 list config、object config + username_map、object without names 三个单元测试。

## 关键知识

Python 的行为不是“只要配置文件存在就继续 fallback”。如果 JSON 是 object 且没有 `names` 字段，`_load_known_people()` 返回空列表；这意味着 fallback 名单不会再参与人名检测。

这个细节必须保留，否则 Rust 会在用户显式配置 username_map-only 时仍然误识别 fallback 名单里的名字，和 Python 不一致。

## 补充知识

`username_map` 的作用是从 transcript 文本中的 `/Users/<username>/` 提取本机用户名，再映射成配置里的姓名。它是一个辅助信号，不要求该姓名也出现在 `names` 列表中。

本次测试不直接写真实 `~/.mempalace/known_names.json`，而是把解析后的 config 注入 `extract_people_with_config`。这样测试不依赖开发机 HOME，也不会污染用户配置。

## 验证

- `cargo fmt --check`
- `cargo test split::tests::people_detection_uses_known_names_list_config -- --exact`
- `cargo test split::tests::people_detection_uses_known_names_object_and_username_map -- --exact`
- `cargo test split::tests::people_detection_object_without_names_matches_python_empty_names -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未给 CLI 增加自定义 known-names path 参数。
- 未缓存 known names config；当前每次 people 检测会读取一次配置文件。
- 未修改 README 或 parity ledger。
