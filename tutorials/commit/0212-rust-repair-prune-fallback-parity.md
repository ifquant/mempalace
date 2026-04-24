# 背景

这次工作发生在 `rust/` 子树，目标是补齐 Rust `repair_prune` 相对 Python 实现还缺的一段维护语义。

Task 2 关注的是一个很窄但真实会误导运维判断的残余差异：

- Rust `repair_prune(true)` 之前只是直接做一次 SQLite 批量删除和一次 LanceDB 批量删除。
- 返回值里的 `failed` 一直写死成 `0`。
- 当 `corrupt_ids.txt` 里排队的是根本不存在的 ID 时，Rust 会把这次 prune 说成“没有失败”，但这和 Python 端的失败感知不一致。

这个差异虽然不影响 happy path 删除，但会让 repair 结果失真，尤其是当队列文件已经陈旧、或者用户手动编辑过 `corrupt_ids.txt` 的时候。

# 主要目标

- 给 Rust `repair_prune` 增加失败可见性，至少能识别“队列里有 ID，但 LanceDB 里并不存在”这种情况。
- 保留批量删除主路径，避免把正常 prune 全部退化成逐条删除。
- 在批量删除真的报错时，补上逐条 fallback，和 Python 版本的处理方式对齐。
- 用聚焦测试覆盖：
  - 缺失 ID 应计入 `failed`
  - 真实存在的 ID 应从 SQLite 和 LanceDB 都删除，且 `failed == 0`

# 改动概览

- 在 `rust/tests/parity_layers_maintenance.rs` 新增：
  - `repair_prune_live_reports_failures_for_ids_missing_from_both_stores`
- 在 `rust/tests/service_integration.rs` 新增：
  - `repair_prune_live_deletes_existing_ids_and_keeps_failure_count_zero`
- 在 `rust/src/maintenance_runtime.rs` 调整 `repair_prune()`：
  - 预先检查每个 queued ID 是否存在于 SQLite / LanceDB
  - SQLite 继续走批量删除；只有批量报错时才逐条 fallback
  - LanceDB 先走批量删除；成功后用预先探测到的存在性来计算真实删除数与失败数
  - 如果 LanceDB 批量删除报错，则逐条 fallback，并把不存在或删除失败的 ID 计入 `failed`
- 新增本教程文件 `tutorials/commit/0212-rust-repair-prune-fallback-parity.md`

# 关键知识

- Python 原始实现只有一个向量存储，所以它的 `failed` 本质上是在描述“prune 队列里有多少 ID 没有被向量层成功删掉”。Rust 多了一层 SQLite，但这不意味着每个“SQLite 不存在”都该单独算失败；否则修剪 vector orphan 时会得到误导性的失败数。
- LanceDB 这里的批量删除接口不会告诉我们“哪些 ID 实际存在”。如果直接信任返回值，缺失 ID 会被误记为成功删除，所以这次实现先做 existence probe，再执行批量删除。
- 这种修复最好保留“批量优先，逐条兜底”的结构。正常路径快，异常路径可诊断，两边都兼顾。

# 补充知识

- 当底层删除 API 对“删除不存在的 ID”表现得过于宽松时，测试就不能只看“调用成功没报错”，还要钉住 summary 里的计数语义，否则运维输出会长期漂移。
- Rust 里做这类多存储层维护逻辑时，先把“哪个层负责定义失败语义”想清楚再编码，比事后补 if/else 更重要。这里最终让 `failed` 主要反映 vector prune 没完成的 ID，才不会破坏已有 drift 修复路径。

# 验证

在 `rust/` 目录运行：

```bash
cargo test --test parity_layers_maintenance repair_prune_live_reports_failures_for_ids_missing_from_both_stores --quiet
cargo test --test service_integration repair_prune_live_deletes_existing_ids_and_keeps_failure_count_zero --quiet
cargo test --test service_integration repair_scan_prune_and_rebuild_handle_vector_drift --quiet
```

结果：

- 缺失 ID 的 parity 测试通过，`failed == 2`
- 真实删除路径测试通过，`deleted_from_sqlite == 1`、`deleted_from_vector == 1`、`failed == 0`
- 既有 repair round-trip 测试通过，没有把 vector drift 修复路径打坏

# 未覆盖项

- 这次没有修改 `python/` 子树，也没有修改 `python/uv.lock`。
- 这次没有修改 `docs/superpowers/`、`docs/rust-python-deep-gap-audit.md`、`docs/rust-python-deep-gap-list.md`。
- 这次没有扩展 CLI / MCP 输出格式，只修正了 `repair_prune` 内部计数语义。
