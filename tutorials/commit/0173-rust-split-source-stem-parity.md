# Commit 0173: Rust split source-stem parity

## 背景

上一轮已经把 Rust transcript split 的 people 检测从“任意大写词”收窄到 Python fallback known people 名单。继续审 split 命名规则时，又发现输出文件名前缀仍有一个小但真实的行为差异。

Python 在生成 split 输出文件名时，会先把原始 mega-file 的 `path.stem` 用 `re.sub(r"[^\w-]", "_", path.stem)` 清理，再截断到 40 个字符。Rust 此前复用了通用 filename sanitizer：它允许点号，并且没有 40 字符截断。因此长文件名或带点号、空格、标点的 source stem 会生成不同前缀。

## 主要目标

- 让 Rust split 输出文件名的 source stem 前缀对齐 Python。
- 保持切片只覆盖 source stem 前缀，不顺手改 subject、known-names 配置或最终 filename collapse 规则。
- 用单元测试固定“先替换、再截断”的 Python 行为。

## 改动概览

- 在 `rust/src/split.rs` 增加 `source_stem_part` helper。
- `source_stem_part` 从 `Path::file_stem()` 读取 stem，缺失时回退为 `session`。
- 清理规则改为 Python 对应的 `[^\w-] -> _`，不允许点号直接保留。
- 在清理后取前 40 个字符，对齐 Python 的 `[:40]`。
- 新增 `source_stem_part_matches_python_split_prefix` 测试，覆盖带点号、空格、标点和长后缀的 source stem。

## 关键知识

这个修正看起来只是命名细节，但它会影响所有从 mega-file 拆出来的输出路径。source stem 前缀的目的，是避免多个 mega-file 产生相同 timestamp/people/subject 时发生文件名碰撞；如果 Rust 不截断，长路径会带来更难读、更不稳定的输出文件名。

这里必须按 Python 的执行顺序写测试：先替换非法字符，再截断。比如连续两个 `!` 会先变成两个 `_`，如果它们刚好落在 40 字符范围内，测试期望也应保留这两个 `_`。

## 补充知识

本次没有调整最终 `sanitize_filename` 的下划线折叠规则，也没有调整 subject 的空格转连字符规则。这些仍属于独立命名细节，应该在后续 split audit 中单独确认是否值得对齐。

保持这种粒度的好处是：每个 commit 只说明一个可验证差异，后续如果发现某个命名差异应作为 intentional divergence，也更容易回滚或改 ledger。

## 验证

- `cargo fmt --check`
- `cargo test split::tests::source_stem_part_matches_python_split_prefix -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未实现 Python `known_names.json` 配置兼容。
- 未调整 split subject 的空格和标点清理规则。
- 未调整最终文件名的连续下划线折叠规则。
- 未修改 Python 实现、README 或 parity ledger。
