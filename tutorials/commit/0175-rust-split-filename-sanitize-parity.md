# Commit 0175: Rust split filename sanitize parity

## 背景

前几轮已经把 transcript split 的 people、source stem 和 subject 三个命名片段分别对齐到 Python。继续检查最终文件名生成时，还剩一个末端差异：Python 在拼出完整文件名后，会执行两步 sanitize。

Python 的最终规则是先把非法字符替换为 `_`，再把连续 `_` 折叠成单个 `_`。Rust 此前只做第一步，并且因为正则使用 `+`，只能折叠连续非法字符生成的下划线，不能折叠已经存在于片段里的连续下划线。

## 主要目标

- 让 Rust split 的最终 filename sanitize 对齐 Python。
- 保持本切片只处理最终 sanitize，不修改前面 source stem、subject、people 的独立规则。
- 用单元测试固定 `source__timestamp` 和空格替换后的下划线折叠行为。

## 改动概览

- 更新 `rust/src/split.rs` 的 `sanitize_filename`。
- 第一步使用 `[^\w\.-]` 将非法字符替换为 `_`。
- 第二步使用 `_+` 将连续下划线折叠为单个 `_`。
- 新增 `sanitize_filename_collapses_underscores_like_python_split` 单元测试。

## 关键知识

Python split 的完整文件名形如 `src_stem__timestamp_people_subject.txt`，但随后会经过 `re.sub(r"_+", "_", name)`。这意味着源码里用于表达分隔意图的双下划线最终会被折叠成单下划线。

Rust 之前的正则 `[^\w\.-]+` 只能把一串非法字符替换成一个 `_`，不能处理字符串里已经存在的 `__`。所以即使 source stem 和 subject 已经单独对齐，最终输出路径仍可能与 Python 不一致。

## 补充知识

这个 helper 也会影响 subject 或 source stem 阶段产生的连续下划线。它是最后一道文件名规范化，不应该承担每个片段自己的语义清理，但必须保证最终文件名和 Python 末端规则一致。

本次没有调整 Python 代码，也没有改变 split 的扫描、写入或备份行为。

## 验证

- `cargo fmt --check`
- `cargo test split::tests::sanitize_filename_collapses_underscores_like_python_split -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未实现 Python `known_names.json` 配置兼容。
- 未修改 README 或 parity ledger。
- 未增加端到端文件名 golden 测试；当前先用 helper 单测锁住最小规则。
