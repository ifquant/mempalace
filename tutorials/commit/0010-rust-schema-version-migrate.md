# 背景

前几次提交已经把 Rust 版的功能面推进了不少：

- CLI 主链路可用
- `fastembed` 路径可跑
- project miner 更接近 Python 语义
- 只读 MCP 也在向 Python shape 对齐

但存储层还缺一个基础设施能力：

- schema version
- migration 入口

如果没有这个层，后面只要 SQLite 结构有一点演进，就会开始出现：

- 老 palace 打不开
- 新老版本状态不清楚
- 迁移逻辑散落在业务代码里

# 主要目标

这次提交的目标是先把 Rust 版 SQLite 存储演进的骨架立起来：

1. 引入明确的 `CURRENT_SCHEMA_VERSION`
2. 让 `init_schema()` 变成真正的迁移入口
3. 支持把旧的 `schema_version = 1` palace 提升到当前版本
4. 把版本号暴露到 `status` 和 MCP `status`

# 改动概览

主要改动如下：

- `rust/src/storage/sqlite.rs`
  - 新增 `CURRENT_SCHEMA_VERSION`
  - `init_schema()` 不再只是“无脑建表”，而是会：
    - 确保 `meta` 表存在
    - 读取当前 `schema_version`
    - 判断是 fresh bootstrap 还是旧版迁移
  - 新增：
    - `schema_version()`
    - `ensure_meta_table()`
    - `bootstrap_schema()`
    - `migrate_v1_to_v2()`
    - `record_migration()`
    - `has_user_tables()`
  - 新增 `schema_migrations` 表，用来记录迁移动作
- `rust/src/model.rs`
  - `Status` 新增 `schema_version`
- `rust/src/service.rs`
  - `status()` 会返回当前 schema version
- `rust/src/mcp.rs`
  - `mempalace_status` 也会回显 `schema_version`
- `rust/tests/service_integration.rs`
  - 新增 `init_migrates_v1_sqlite_schema_to_current`
  - 直接构造一个 v1 SQLite 文件，再验证 `init()` 会把它提升到当前版本
- `rust/README.md`
  - 补充 schema version / migration 骨架说明

# 关键知识

## 1. migration 的第一步不是“复杂升级逻辑”，而是先有稳定入口

很多项目第一次做迁移时会想直接上：

- 多版本脚本
- 回滚
- 数据修复
- 复杂 diff

但真正第一步应该是：

- 有统一入口
- 有版本号
- 有迁移记录

这样后面每次升级才有挂载点。  
这次提交的重点就是先把这个挂载点固定下来。

## 2. fresh bootstrap 和旧库迁移要明确区分

新建 palace 和升级旧 palace，看起来都叫“初始化”，但语义完全不同：

- fresh bootstrap：从 0 建当前结构
- migration：保留已有数据，逐版升级

如果这两条路径混在一起，代码很快就会难维护。  
所以这次把：

- `bootstrap_schema()`
- `migrate_v1_to_v2()`

明确拆开了。

# 补充知识

## 为什么这次 migration 先只升到一个非常轻的 v2

因为当前目标是先建立演进机制，而不是马上做大规模 schema 改造。  
这次的 v2 主要增加的是：

- `schema_migrations` 表
- 更明确的 `schema_version` 管理

这样风险低，但价值很高：后面再加新列、新索引或新表时，就不用重新发明迁移流程。

## 为什么把 `schema_version` 放进 `status`

因为它不只是内部实现细节，也会影响：

- 调试
- 兼容性排查
- 用户现场判断“这个 palace 是不是旧格式”

把它放到 `status` 和 MCP `status`，可以让 CLI 和 agent 都第一时间看到当前库版本。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增关键验证：

- `init_migrates_v1_sqlite_schema_to_current`

这个测试会：

1. 手工创建一个旧的 v1 SQLite palace
2. 调用 Rust `init()`
3. 断言 `schema_version` 被提升到 `CURRENT_SCHEMA_VERSION`

# 未覆盖项

这次没有继续做：

- CLI 级别的 `migrate` / `repair` 命令
- LanceDB 侧的 schema version 管理
- 多步迁移链（v2 -> v3 -> v4）
- 回滚和损坏修复

所以这次提交的定位是：  
先把 Rust 版 SQLite 的版本管理和迁移入口立住，为后续真实 schema 演进打底。
