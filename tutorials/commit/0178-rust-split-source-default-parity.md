# Commit 0178: Rust split source-default parity

## 背景

上一轮补齐了 Python split 的 `--file` 单文件入口。继续对照 Python `split_mega_files.py` 的 CLI 入口语义时，还剩两个调用方式差异：

- Python 支持 `--source` 指定扫描目录。
- Python 在没有传 source/file 时，会默认使用 `MEMPALACE_SOURCE_DIR`，否则使用 `~/Desktop/transcripts`。

Rust 此前只支持 positional directory，并且必须提供目录。这样虽然保留了 Rust 风格入口，但没有完全覆盖 Python 用户习惯。

## 主要目标

- 给 Rust `split` 增加 Python 对应的 `--source` 参数。
- 允许 `mempalace-rs split --dry-run` 这类无目录调用走默认 source 规则。
- 保留既有 `mempalace-rs split <dir>` 入口。
- 不改变 MCP `mempalace_split`，MCP 仍然要求显式 `source_dir`。

## 改动概览

- 更新 `rust/src/root_cli.rs`，让 `split` 的 positional `dir` 变成可选，并新增 `--source`。
- 更新 `rust/src/cli_runtime.rs` 和 `rust/src/project_cli.rs`，传递 `source` 参数。
- 更新 `rust/src/project_cli_transcript_split.rs`，解析优先级为：
  - 有 `--file` 时走单文件模式。
  - 否则优先使用 positional `dir`。
  - 没有 positional `dir` 时使用 `--source`。
  - 两者都没有时使用 `MEMPALACE_SOURCE_DIR` 或 `~/Desktop/transcripts`。
- 更新 split help 集成测试，覆盖 `--source` 和默认目录说明。
- 新增 `cli_split_source_flag_matches_python_source_option` 集成测试。
- 新增 `cli_split_defaults_to_mempalace_source_dir_env` 集成测试。

## 关键知识

Python 的默认 source 是在模块加载时通过：

```python
LUMI_DIR = Path(os.environ.get("MEMPALACE_SOURCE_DIR", str(HOME / "Desktop/transcripts")))
```

Rust 侧对应放在 CLI handler 层，而不是 split library 层。这样 `split::split_directory` 仍然是明确的库 API，默认目录只属于 CLI 行为。

## 补充知识

本次没有把 `--source` 加进 MCP schema，因为 MCP 工具已经明确叫 `source_dir`，没有 Python argparse 兼容问题。MCP schema 变更应该单独做，并配套 MCP integration test。

如果用户同时传 positional `dir` 和 `--source`，Rust 会优先使用 positional `dir`。这是为了不破坏现有 Rust CLI 习惯；Python 本身没有 positional dir，所以不存在对应冲突语义。

## 验证

- `cargo fmt --check`
- `cargo test cli_split_source_flag_matches_python_source_option --test cli_integration -- --exact`
- `cargo test cli_split_defaults_to_mempalace_source_dir_env --test cli_integration -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未给 MCP `mempalace_split` 增加 `file` 或 `source` 参数。
- 未改变 split library API 的目录默认值。
- 未实现 Python `known_names.json` 配置兼容。
- 未修改 README 或 parity ledger。
