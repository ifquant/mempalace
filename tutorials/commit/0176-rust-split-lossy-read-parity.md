# Commit 0176: Rust split lossy-read parity

## 背景

前几轮已经把 transcript split 的输出命名规则逐步对齐。继续审实际读取路径时，发现 Python 和 Rust 对损坏文本字节的处理不同。

Python split 使用 `path.read_text(errors="replace")`，遇到无效 UTF-8 字节时会用替换字符继续处理。Rust 此前使用 `fs::read_to_string`，遇到同样的坏字节会直接返回错误，导致整个 split 失败。

## 主要目标

- 让 Rust split 像 Python 一样容忍 transcript 中的少量无效 UTF-8 字节。
- 保持扫描、边界识别、命名和备份行为不变。
- 用目录级测试覆盖 `split_directory` 的扫描阶段和 `split_file` 的实际拆分阶段都能走通。

## 改动概览

- 在 `rust/src/split.rs` 增加 `read_text_lossy` helper。
- `split_directory` 扫描 mega-file 时改用 lossy 读取。
- `split_file` 实际拆分文件时也改用 lossy 读取。
- 新增 `split_directory_tolerates_invalid_utf8_like_python` 测试，构造包含 `0xff` 坏字节的 transcript，并验证 dry-run split 仍能识别并产出两个 session。

## 关键知识

Rust 的 `fs::read_to_string` 是严格 UTF-8 读取；只要文件里有一个非法字节，读取就会失败。Python 的 `read_text(errors="replace")` 更适合处理真实 transcript 导出，因为这些文件可能来自复制、终端、浏览器或外部工具，偶发坏字节不应该阻止拆分。

Rust 中对应的实现方式是先 `fs::read` 得到 bytes，再用 `String::from_utf8_lossy` 转成字符串。这样无效字节会变成替换字符，session header、prompt 和普通文本仍可继续被解析。

## 补充知识

本次修正只改变输入容错，不改变输出写入编码。split 输出仍由 Rust 写成 UTF-8 文本；如果原始文件含坏字节，输出里会保留替换后的 Unicode replacement character。

测试选择通过 `split_directory(..., dry_run=true)` 覆盖，是因为它同时经过目录扫描和单文件 split 两个读取点，而且不会产生实际输出文件或重命名 backup。

## 验证

- `cargo fmt --check`
- `cargo test split::tests::split_directory_tolerates_invalid_utf8_like_python -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未改变非 split 路径的文本读取策略。
- 未实现 Python `known_names.json` 配置兼容。
- 未修改 README 或 parity ledger。
