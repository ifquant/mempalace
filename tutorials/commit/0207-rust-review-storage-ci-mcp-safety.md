# Commit 0207: Rust review storage, CI, and MCP safety fixes

## 背景

本片修复 review 发现的三个真实风险：

- 手工 `add_drawer` / `delete_drawer` 可能让 SQLite 与 LanceDB 状态不一致。
- 仓库 CI 只覆盖 Python，没有覆盖新增的 Rust crate。
- MCP `mempalace_dedup` 默认会真实删除，agent 调用时安全边界过低。

这些问题都不是“再补一个功能”，而是把已经存在的 Rust 能力面变得更可托付。

## 主要目标

- 降低手工 drawer 写入和删除的跨存储不一致风险。
- 让 GitHub CI 对 Rust 编译、测试、格式和 clippy 建立基础门禁。
- 让 MCP dedup 默认进入 dry-run，避免省略参数时直接删除。
- 用集成测试固定新的安全默认值和失败路径行为。

## 改动概览

- 更新 `rust/src/palace_ops.rs`。
- `add_drawer` 先检查 SQLite 是否已有记录，再生成 embedding 并写 LanceDB，最后写 SQLite。
- 如果 SQLite 最终写入失败，会尽力删除刚写入的 LanceDB 向量作为补偿。
- `delete_drawer` 先读取 SQLite 记录，再删除 LanceDB，最后删除 SQLite。
- 如果 SQLite 删除失败，会尽力把向量记录补回 LanceDB。
- 更新 `rust/src/storage/sqlite_drawers.rs`。
- 新增 `get_drawer()`，用于删除前拿到可补偿恢复的完整 drawer 记录。
- 更新 `rust/src/mcp_runtime_write.rs` 和 `rust/src/mcp_schema_catalog_write.rs`。
- `mempalace_dedup` 在 MCP 中默认 `dry_run=true`，schema 文案同步说明。
- 更新 `.github/workflows/ci.yml`。
- 新增 Rust CI job，运行 `cargo fmt --check`、`cargo check`、`cargo test`、`cargo clippy`。
- 更新 `rust/tests/service_integration.rs` 和 `rust/tests/mcp_integration.rs`。
- 新增 embedding 失败不落 SQLite 的测试。
- 新增 MCP dedup 省略 `dry_run` 时默认 preview 的测试。

## 关键知识

SQLite 和 LanceDB 不是同一个事务系统，所以不能假装它们能原子提交。本片采用的是低侵入补偿策略：先避免最明显的“SQLite 已写但向量失败”，再对少数后置失败做 best-effort rollback。

MCP 工具的默认值要比 CLI 更保守。CLI 是人直接操作，MCP 往往由 agent 调用；省略参数时执行真实删除，风险比普通命令行更高。

## 补充知识

`std::slice::from_ref(&drawer)` 可以把单个引用临时视为单元素 slice，避免为了调用批量 API 而 clone 整个 `DrawerInput`。

CI 里 Rust job 安装 `protobuf-compiler`，是因为依赖链在干净 Linux runner 上可能需要 `protoc`。本地机器已经装过 protobuf 不代表 GitHub runner 也有。

## 验证

- `cargo fmt --check`
- `cargo test --test service_integration manual_add_does_not_insert_sqlite_when_embedding_fails --quiet`
- `cargo test --test mcp_integration mcp_dedup_defaults_to_dry_run --quiet`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未改变 CLI dedup 默认行为。
- 未新增复杂的跨存储 pending repair schema。
- 未修改 hooks、Python CI job、README 或 parity ledger。
