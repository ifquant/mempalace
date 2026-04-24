# 背景

这次工作发生在 `rust/` 子树，目标是补齐 Rust knowledge graph 相对 Python 还剩下的两类残余差异：

- Rust 之前只有 `kg_triples`，没有显式实体表，也没有明确的 entity upsert 入口。
- Rust 之前每次 `add_kg_triple()` 都会新插一行，所以同一条仍然 active 的事实重复写入时，不会复用已有 triple。

这两个问题叠在一起，会让 Rust 端的 KG 统计和写入语义偏离 Python：实体是“从三元组现算出来的”，而不是持久状态；重复写同一条 active fact 也会不断膨胀。

# 主要目标

- 增加显式的 KG entity upsert API。
- 为 SQLite schema 增加持久化 `kg_entities` 表，并把 schema 版本推进到 `v9`。
- 让 `add_kg_triple()` 自动补齐 subject / object 实体。
- 让重复写入同一条 active triple 时复用已有 triple ID，而不是再插入一条新记录。
- 用聚焦测试先复现，再验证修复后的语义。

# 改动概览

- 在 `rust/tests/parity_registry_kg_ops.rs` 新增两个聚焦测试：
  - `kg_add_entity_normalizes_id_and_upserts`
  - `kg_duplicate_active_triple_returns_existing_id`
- 在 `rust/src/model_ops.rs` 新增 `KgEntityWriteResult`，避免把实体写入结果硬塞进 `KgWriteResult`。
- 在 `rust/src/knowledge_graph.rs` 新增 `KnowledgeGraph::add_entity()` 公共入口。
- 在 `rust/src/storage/sqlite.rs` 把 `CURRENT_SCHEMA_VERSION` 从 `8` 提升到 `9`。
- 在 `rust/src/storage/sqlite_schema.rs`：
  - bootstrap fresh schema 时创建 `kg_entities`
  - 新增 `migrate_v8_to_v9()`
  - 为已有 `kg_triples` 做一轮 entity backfill
- 在 `rust/src/storage/sqlite_kg.rs`：
  - 增加 `add_kg_entity()`
  - `add_kg_triple()` 先 upsert subject / object
  - 如果已存在同一条 active triple，则直接返回已有 row 对应的 triple ID
  - KG stats 的 `entities` 改为读 `kg_entities`
  - entity ID 归一化保留 `.`，从而与 `Dr. Chen -> dr._chen` 这类 Python 语义对齐

# 关键知识

- “实体表”和值得保留的“实体统计”是两回事。之前 Rust 用 `subject UNION object` 现算实体数，虽然能得到一个数字，但它不是 durable state，也无法承载 entity upsert 语义。
- duplicate-active-triple reuse 的关键不是“不重复显示”，而是“不重复写入”。如果存储层仍然插入新行，后面的 stats、timeline、invalidations 都会逐渐偏离。
- triple ID 如果带时间哈希，就天然不可能复用。这里把 triple ID 改成基于 SQLite row id 的稳定形状，才能让“已有 active fact 再写一次”返回同一个 ID。

# 补充知识

- 做 schema parity 时，`bootstrap_schema()` 和增量 migration 要一起改。只改 migration 会让新库和老库行为不一致；只改 bootstrap 会让升级路径断裂。
- 这类 residual parity 很适合用“一个缺失 API + 一个重复写入路径”两条小测试钉住，因为它们能直接约束存储层语义，而不是只验证上层输出。

# 验证

在 `rust/` 目录运行：

```bash
cargo test --test parity_registry_kg_ops kg_add_entity_normalizes_id_and_upserts --quiet
cargo test --test parity_registry_kg_ops kg_duplicate_active_triple_returns_existing_id --quiet
cargo test --test parity_registry_kg_ops --quiet
```

结果：

- 两个新增 KG parity 测试先失败后通过
- `parity_registry_kg_ops` 全量 8 个测试通过

# 未覆盖项

- 这次没有修改 `python/` 子树逻辑，也没有碰 `python/uv.lock`。
- 这次没有修改 `docs/superpowers/`、`docs/rust-python-deep-gap-audit.md`、`docs/rust-python-deep-gap-list.md`。
- 这次没有扩展 CLI / MCP 接口，也没有改 `service_integration`。
- 仓库里已有的 `rust/src/maintenance_runtime.rs` 和 `rust/src/registry_lookup.rs` 非本任务改动保持原样，没有被回退或混入本次提交。
