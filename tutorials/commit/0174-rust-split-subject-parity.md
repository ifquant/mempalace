# Commit 0174: Rust split subject parity

## 背景

继续沿 transcript split 的输出命名规则做行为级审计时，source stem 前缀已经和 Python 对齐，但 subject 片段仍有差异。

Python 会从第一条有意义的 `> ` prompt 中生成 subject：先删除标点，再把连续空白折成连字符。Rust 此前直接复用通用 filename sanitizer，会把空格和标点都变成下划线。因此同一条 prompt 在 Python 和 Rust 下会生成不同的文件名摘要。

## 主要目标

- 让 Rust split 的 subject 片段清理方式对齐 Python。
- 保持当前切片只覆盖 prompt subject，不改变 session boundary、timestamp、people 或 source stem 规则。
- 用单元测试锁住标点删除与空白转连字符的组合行为。

## 改动概览

- 在 `rust/src/split.rs` 增加 `subject_part` helper。
- `extract_subject` 在找到有效 prompt 后调用 `subject_part`，不再复用通用 `sanitize_filename`。
- `subject_part` 先用 `[^\w\s-]` 删除标点，再用 `\s+` 将空白折成 `-`。
- subject 最终仍截断到 60 个字符，对齐 Python 的 `subject[:60]`。
- 新增 `subject_part_matches_python_split_prompt_cleanup` 单元测试。

## 关键知识

split 输出文件名由多个片段拼接而成，不同片段的清理规则并不完全一样。source stem、subject、最终 filename sanitize 在 Python 里是三个独立步骤；如果 Rust 为了省事复用同一个 sanitizer，就会产生看似无害但持续扩散的命名差异。

本次修正的核心是保留 Python 的 subject 语义：prompt 里的标点用于阅读，不进入文件名摘要；词之间的空白用连字符表示，而不是下划线。

## 补充知识

最终文件名还会经过一次通用 sanitize。因此 subject helper 不需要负责所有文件名安全问题，只需要生成与 Python subject 阶段一致的片段。

这种分层写法也让后续审计更容易：如果最终 filename 的下划线折叠还需要对齐，可以单独改最终 sanitize，而不会混进 subject 行为。

## 验证

- `cargo fmt --check`
- `cargo test split::tests::subject_part_matches_python_split_prompt_cleanup -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未调整最终 filename sanitize 的连续下划线折叠。
- 未实现 Python `known_names.json` 配置兼容。
- 未修改 Python 实现、README 或 parity ledger。
