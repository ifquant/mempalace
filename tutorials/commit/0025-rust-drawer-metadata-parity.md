# 0025 Rust drawer 元数据继续向 Python 对齐

这次继续补的不是 CLI 表面，而是 project miner 真正写进存储层的 drawer 元数据。

## 做了什么

- `DrawerInput` 新增：
  - `source_file`
  - `source_mtime`
  - `added_by`
  - `filed_at`
- SQLite `drawers` 表 schema 升到 `v4`
- 新 schema 会持久化：
  - `source_file`
  - `source_mtime`
  - `added_by`
  - `filed_at`
- 旧 `v3` palace 现在会迁移到 `v4`
- migration 逻辑也顺手改成了“逐步升级”，避免后面 schema 再增长时跳步出错

## 为什么这样做

Python `miner.py` 在每个 drawer 上一直都有这些项目元数据：

- 文件名
- 写入人
- 写入时间
- 文件修改时间

Rust 之前虽然已经有：

- `source_path`
- `source_hash`
- `chunk_index`

但缺了这几个更贴近 Python 行为的字段。这样会造成一个问题：

- summary 看起来差不多
- 但真正落进 palace 的 drawer metadata 还没对齐

这次就是把这个差距补上。

## 验证

通过的验证：

```bash
cd rust && cargo fmt --check
cd rust && cargo clippy --all-targets --all-features -- -D warnings
```

以及两条真实手工验证：

1. 用 `mempalace-rs mine --agent codex` 实际写入后，直接查 SQLite：

```sql
select source_file, added_by, source_mtime is not null, length(filed_at) > 0 from drawers limit 1;
```

结果拿到了：

```text
auth.txt|codex|1|1
```

2. 构造一个 `schema_version = 3` 的旧 palace，执行 `migrate` 后再查：

```sql
select value from meta where key='schema_version';
select source_file, added_by, filed_at from drawers limit 1;
```

结果拿到了：

```text
4
/tmp/file.txt|mempalace|2026-04-12T00:00:00Z
```

## 说明

这轮没有把这些字段同时写进 LanceDB 向量表。原因很实际：

- 目前 Rust 的 search 主链路不依赖这些字段
- 先把 SQLite 这层的项目 drawer metadata 对齐，风险更低
- 向量表 schema 变更和迁移更重，适合单独一片处理

## 新手知识点

数据库 schema 演进里，一个很常见的坑是：

- 你把 `CURRENT_SCHEMA_VERSION` 改大了
- 但旧版本迁移函数还直接把版本号写成“当前最新”

这样会导致中间版本迁移被跳过。

更稳的做法是：

1. 每个迁移函数只负责一步
2. 版本号一步一步推进
3. `init_schema()` 循环跑到当前版本为止

这次也顺手把这件事修正了。
