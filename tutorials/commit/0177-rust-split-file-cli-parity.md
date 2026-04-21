# Commit 0177: Rust split file CLI parity

## 背景

前几轮已经把 Rust transcript split 的内部行为逐步对齐到 Python：people 检测、source stem、subject、最终 filename sanitize，以及损坏 UTF-8 的容错读取都已收口。

继续对照 Python `split_mega_files.py` 的 CLI 入口时，发现 Python 支持 `--file`，可以只拆一个指定 mega-file，而不是扫描整个目录。Rust CLI 此前只有目录参数，无法表达这个常用操作。

## 主要目标

- 给 Rust `split` CLI 增加 `--file` 单文件入口。
- 保持原有目录扫描入口继续可用。
- 不改变 MCP 的 `mempalace_split` schema；MCP 仍以 `source_dir` 目录扫描为主。
- 用 unit test 和 CLI integration test 同时锁住单文件模式不会扫描同目录其他文件。

## 改动概览

- 在 `rust/src/split.rs` 增加 `split_single_file`，复用现有 `split_file` 拆分逻辑。
- `split_single_file` 只检查指定文件是否超过大小上限、是否达到 `min_sessions`，不会扫描同目录其他 `.txt`。
- 更新 `rust/src/root_cli.rs`，让 `split` 的目录参数在提供 `--file` 时可省略，并新增 `--file` 帮助文案。
- 更新 `rust/src/cli_runtime.rs`、`rust/src/project_cli.rs` 和 `rust/src/project_cli_transcript_split.rs`，把 `file` 参数传到 transcript split handler。
- 新增 `split_single_file_limits_scan_to_requested_file` 单元测试。
- 新增 `cli_split_file_mode_limits_scan_to_requested_file` 集成测试。

## 关键知识

Python 的 `--file` 不是“先指定目录再过滤一个文件”，而是直接把待处理列表设为 `[Path(args.file)]`。这意味着同目录下其他 mega-file 不应该被扫描或计入结果。

Rust 侧保留同样语义：只要传了 `--file`，就走 `split_single_file`；没有 `--file` 时才走原来的 `split_directory`。

## 补充知识

本次没有扩展 MCP，是刻意的边界控制。Python 的 `--file` 是 CLI 便利入口；MCP 当前 schema 已经稳定为 `source_dir`、`output_dir`、`min_sessions`、`dry_run`，如果要给 MCP 增加单文件模式，应单独做 schema 变更和 MCP 测试。

`split` 的目录参数现在标记为 `required_unless_present = "file"`。因此原有 `mempalace-rs split <dir>` 继续有效，新增的 `mempalace-rs split --file <path>` 也有效。

## 验证

- `cargo fmt --check`
- `cargo test split::tests::split_single_file_limits_scan_to_requested_file -- --exact`
- `cargo test cli_split_file_mode_limits_scan_to_requested_file --test cli_integration -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未给 MCP `mempalace_split` 增加 `file` 参数。
- 未实现 Python `known_names.json` 配置兼容。
- 未修改 README 或 parity ledger。
