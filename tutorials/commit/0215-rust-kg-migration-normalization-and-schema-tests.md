# 背景

这次提交是上一笔 KG residual parity 提交之后的一个 review-fix。

上一笔改动已经把 Rust knowledge graph 的主能力补上了：

- 显式 `kg_entities` 表
- `add_entity()` upsert
- active triple 重复写入复用已有 ID

但 review 时又发现了两个容易在后续全量验证里出问题的细节：

1. `migrate_v8_to_v9()` 的 `kg_entities` backfill 归一化没有完全跟 `normalize_entity_id()` 保持一致，至少 `-` 还没转成 `_`
2. `rust/tests/cli_integration.rs` 里还保留着旧的 `"schema_version": 7` / `"schema_version_after": 7` 硬编码断言

这两个问题都不属于新功能，但都属于“如果不修，后面会炸”的 review-fix。

# 主要目标

- 让 `kg_entities` migration backfill 的 ID 归一化和当前 Rust KG 运行时规则更接近一致
- 清掉 CLI integration 里过时的 schema version 常量断言
- 把这次 review-fix 独立成一笔小提交，不和后续 parity family 混在一起

# 改动概览

这次只改了 3 个文件：

1. `rust/src/storage/sqlite_schema.rs`
2. `rust/tests/cli_integration.rs`
3. `tutorials/commit/0215-rust-kg-migration-normalization-and-schema-tests.md`

具体改动有两点：

1. `migrate_v8_to_v9()` 里 `kg_entities.entity_id` 的 SQL backfill 现在从：

```sql
lower(replace(entity, ' ', '_'))
```

变成：

```sql
lower(replace(replace(entity, ' ', '_'), '-', '_'))
```

这样至少和 Rust 运行时的 `normalize_entity_id()` 在空格、连字符这两个最常见路径上保持一致。

2. `rust/tests/cli_integration.rs` 里把写死的 `7` 改成读取：

```rust
use mempalace_rs::storage::sqlite::CURRENT_SCHEMA_VERSION;
```

这样后续再升级 schema 时，CLI integration 不会因为陈旧常量而白白失败。

# 关键知识

## 1. migration backfill 和运行时归一化必须尽量一致

如果 migration 用一套规则，运行时写入又用另一套规则，就会出现这种问题：

- 老数据升级后得到一个 entity_id
- 新数据运行时写入时又得到另一个 entity_id
- 结果同一个实体被拆成两份

这类 bug 通常不会立刻出现在 happy path，但会慢慢污染统计和查找。

## 2. schema version 不该在测试里硬编码魔法数字

一旦仓库还在持续演进 schema，测试里把 `7`、`8` 这种数字写死，迟早就会变成无意义失败。

更稳的做法是直接引用：

```rust
CURRENT_SCHEMA_VERSION
```

这样测试验证的是“CLI 输出和当前仓库 schema 一致”，而不是“今天刚好等于某个旧数字”。

# 补充知识

## 1. review-fix 也值得单独提交

很多时候 review-fix 看起来只是两三行，但它解决的是“刚合进去就会埋雷”的问题。

把它单独提交有两个好处：

1. 历史上能看出这是主提交后的纠偏
2. 不会把后续更大的 parity family 实现和这个小修缠在一起

# 验证

这次实际做的验证：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo test --test parity_registry_kg_ops kg_add_entity_normalizes_id_and_upserts --quiet
cargo test --test cli_integration cli_migrate_upgrades_legacy_sqlite_schema --quiet
```

# 未覆盖项

- 这次没有修改 `python/`、`python/uv.lock`
- 这次没有修改 `docs/superpowers/`、`docs/rust-python-deep-gap-audit.md`、`docs/rust-python-deep-gap-list.md`
- 这次没有扩展新的 KG API，只修正了 migration backfill 和过时测试断言
- 这次没有推进 Task 5 的 CLI / MCP / normalize residual parity 主体实现
