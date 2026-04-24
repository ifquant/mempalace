# Commit 0172: Rust split known-people parity

## 背景

这一轮进入 `docs/parity-ledger.md` 里保留的 `Deeper non-CLI behavior audit`。README/help/tests 的表层一致性已经收口，后续需要从真实行为路径里挑小切片做对齐，避免继续只修文档口径。

本次选中 transcript split 的人名提取规则。Python 版本不会把所有首字母大写词都当成人名，而是先用固定 fallback known people 名单识别常见参与者；Rust 版本此前会从前 100 行中抓取任意大写词，容易把 `Claude`、`Code`、`Monday`、`April` 这类 transcript header 或日期词写进输出文件名。

## 主要目标

- 让 Rust split 的默认人名检测更接近 Python fallback 行为。
- 保持本切片很窄：只修默认 fallback known people，不引入 Python 的 `known_names.json` 配置兼容。
- 用单元测试锁住“不要把普通大写词当人名”的边界。

## 改动概览

- 在 `rust/src/split.rs` 增加 `FALLBACK_KNOWN_PEOPLE`，名单与 Python fallback 一致：`Alice`、`Ben`、`Riley`、`Max`、`Sam`、`Devon`、`Jordan`。
- 将 `extract_people` 从泛化正则 `\b([A-Z][a-z]+)\b` 改为逐个匹配 fallback known people，并使用大小写不敏感的 word-boundary 匹配。
- 对检测结果排序，保持 Python `sorted(found)` 的输出语义。
- 文件名中的 people 段只取前三个名字，对齐 Python 的 `people[:3]` 拼接行为。
- 新增 `people_detection_uses_python_fallback_names` 单元测试，覆盖 transcript header/date 中的大写词不会进入人名结果。

## 关键知识

Python 的 split 行为不是“看到大写词就提取”。它先加载 `~/.mempalace/known_names.json`，失败时才使用 fallback known people 名单；fallback 模式下只有名单中的名字会进入输出文件名。

这点很重要，因为 transcript 的前几十行天然包含大量首字母大写词，例如产品名、星期、月份和标题。如果 Rust 使用宽泛大写词正则，输出文件名会看起来“信息丰富”，但实际上会污染 session 命名，也会和 Python 默认行为不一致。

## 补充知识

本次没有实现 Python 的 `known_names.json` 兼容。原因是这属于配置读取与本地目录语义，不是当前最小行为修正所必需的内容；直接引入可能会把“默认 fallback parity”扩大成“旧 Python 用户配置兼容”。

当前收口原则是：先锁住不会误识别的大行为边界，再决定配置兼容是否真的属于 remaining。如果后续需要做，可作为单独切片记录在 parity ledger 中。

## 验证

- `cargo fmt --check`
- `cargo test split::tests::people_detection_uses_python_fallback_names -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未实现 Python `~/.mempalace/known_names.json` 的 `names` / `username_map` 配置读取。
- 未调整 transcript split 的其他命名细节，例如 source stem 截断。
- 未修改 Python 实现、README 或 parity ledger。
