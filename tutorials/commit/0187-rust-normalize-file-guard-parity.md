# Commit 0187: Rust normalize file guard parity

## 背景

继续审 `normalize.py` 的文件入口行为时，发现 Python 在读文件前有两个稳定边界：

- 文件超过 500MB 时直接报错。
- 正常读文件时使用 replacement 模式，遇到非法 UTF-8 字节不会失败。

Rust core 已经改成 lossy 读取，但 CLI `normalize` 入口仍然先用 `read_to_string` strict 读取一次，导致 CLI 仍可能因为非法 UTF-8 提前失败。

## 主要目标

- 让 Rust normalize 文件入口补上 Python 的 500MB size guard。
- 让 CLI 和 core 共用同一套 lossy 读取逻辑。
- 保持 MCP/miner 调用 `normalize_conversation_file` 的既有 API 不变。

## 改动概览

- 更新 `rust/src/normalize.rs`。
- 新增 `NormalizeFileOutput`，让需要 raw 的 CLI 可以通过 core 获取原始 lossy 文本和 normalized 结果。
- 新增 500MB 文件大小检查，错误信息包含 `File too large` 和文件路径。
- 更新 `rust/src/project_cli_transcript_normalize.rs`，移除 CLI 层的 `read_to_string`。
- 更新 `rust/tests/cli_integration.rs`，覆盖 CLI 对非法 UTF-8 的 lossy 兼容行为。

## 关键知识

同一个功能入口不要在不同层重复读文件。CLI 之前先 strict 读 raw、core 再 lossy 读 normalized，两个入口规则已经分叉。

这类分叉很容易造成“库测试已通过，用户 CLI 仍失败”的假对齐。因此本次把文件读取规则收回 normalize core，CLI 只消费 core 返回的 raw/normalized。

## 补充知识

Python 的 `errors="replace"` 对非法 UTF-8 的效果对应 Rust 的 `String::from_utf8_lossy`。它会把非法字节替换成 Unicode replacement character，而不是让 normalize 失败。

500MB guard 用 metadata 长度判断，不需要真的读入大文件。测试通过 `set_len` 创建 sparse file，避免写入真实 500MB 内容。

## 验证

- `cargo fmt --check`
- `cargo test normalize::tests::normalize_file_rejects_files_over_python_size_limit -- --exact`
- `cargo test cli_normalize_tolerates_invalid_utf8_like_python --test cli_integration -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 MCP schema、split、registry、layers 或 maintenance 能力面。
