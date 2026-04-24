# 背景

这次工作发生在 `rust/` 子树，目标是收口 Rust 重写里 Layer1 读侧仍然落后于 Python 的残余差异。

计划里确认了两个用户可见问题：

- Layer1 没有按 Python 那样先看 `importance`，再用默认值 `3.0` 排序。
- Layer1 没有全局字符上限，也不会在超限时提示 `more in L3 search`。

另外，既然排序语义依赖权重，SQLite 里的 `drawers` 也需要有一个稳定的 `importance` 字段，否则读侧只能继续靠临时猜测。

# 主要目标

- 给 Rust Layer1 补上 Python 风格的权重排序语义。
- 给 Rust Layer1 补上全局字符上限和溢出提示。
- 给 SQLite `drawers` schema 增加可持久化的 `importance` 列，并让读路径能读出来。
- 用聚焦 parity 测试把这两个行为钉住。

# 改动概览

- 在 `rust/tests/parity_layers_maintenance.rs` 新增两个 Layer1 parity 测试：
  - `layer1_stops_at_python_style_global_char_cap`
  - `layer1_prefers_importance_then_weight_defaulting_to_three`
- 在 `rust/src/storage/sqlite.rs` 的 `DrawerRecord` 增加 `importance: Option<f64>`，并把 schema 版本提升到 `8`。
- 在 `rust/src/storage/sqlite_schema.rs`：
  - fresh bootstrap 的 `drawers` 表新增 `importance REAL`
  - 增加 `migrate_v7_to_v8()`，对已有 palace 执行 `ALTER TABLE drawers ADD COLUMN importance REAL`
- 在 `rust/src/storage/sqlite_drawers.rs`：
  - 所有 drawer 查询都把 `importance` 读出来
  - `insert_drawer()` 和 `replace_source()` 都会持久化 `drawer.importance`
- 在 `rust/src/model_palace.rs`、`rust/src/drawers.rs`、`rust/src/miner_support.rs`、`rust/src/miner_project.rs`：
  - `DrawerInput` 新增 `importance: Option<f64>`
  - 现有 drawer 构造路径默认填 `None`
  - 从 `DrawerRecord` 回转成 `DrawerInput` 时会保留该字段
- 在 `rust/src/layers.rs`：
  - 先按 `importance.unwrap_or(3.0)` 倒序选出前 `12` 个 drawer
  - 按 room 分组输出
  - 按 Python 风格维护 `MAX_CHARS = 3200` 的全局预算
  - 超限时输出 `  ... (more in L3 search)` 并立即停止
- 因为 `DrawerRecord` 新增字段，`rust/src/compress.rs` 和 `rust/src/dedup.rs` 里的测试辅助构造器补了 `importance: None`，否则目标测试无法通过编译。

# 关键知识

- 这次 Layer1 的关键不是“每条 snippet 截断”，而是“整个 L1 文本有总预算”。Python 版本会先限制单条，再限制总量；Rust 之前只做了前者。
- `Option<f64>` 很适合这里的 schema 演进：旧数据天然是 `NULL`，读侧用 `unwrap_or(3.0)` 就能兼容旧 palace 和新 palace。
- 这种 parity 修复最好先写失败测试再改实现。这里第一轮失败就直接暴露出 `DrawerRecord` 还没有 `importance` 字段，避免了把问题误判成纯渲染逻辑。

# 补充知识

- Rust integration test `cargo test --test ...` 仍然可能被 crate 内部 `#[cfg(test)]` 单元测试挡住编译，所以新增结构体字段时，别只盯着目标测试文件本身。
- 当计划里的测试样例在真实实现里没有真正触发边界时，要及时调整 fixture，而不是硬改实现去迎合假阳性。这里把 `source_file` 做长，才真实触发了 3200 字符预算分支。

# 验证

在 `rust/` 目录运行：

```bash
cargo test --test parity_layers_maintenance --quiet
cargo test layer_renderers_match_python_style_text --quiet
```

结果：

- `parity_layers_maintenance`：5 个测试全部通过
- `layer_renderers_match_python_style_text`：通过

# 未覆盖项

- 这次没有改 `python/` 实现，也没有修改 `python/uv.lock`。
- 这次没有改 `docs/superpowers/` 下的计划和审计文档。
- 这次没有动其它 parity 家族，比如 maintenance / registry / knowledge graph / CLI / MCP。
