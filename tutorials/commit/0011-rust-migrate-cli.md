# 背景

上一提交已经把 Rust 版 SQLite 的：

- `schema_version`
- `schema_migrations`
- `init_schema()` 迁移入口

这些内部能力立住了。

但如果这些能力只存在于内部 API，实际运维时仍然不够方便，因为用户和 agent 还缺一个正式入口去做：

- 查看旧 palace 能不能被升级
- 显式执行一次升级
- 拿到结构化结果

# 主要目标

这次提交的目标很明确：

1. 新增 `migrate` CLI 命令
2. 把已有的 SQLite 迁移逻辑包装成正式 service API
3. 给 `migrate` 输出稳定 JSON
4. 用 CLI 集成测试覆盖一次真实的 v1 -> v2 升级

# 改动概览

主要改动如下：

- `rust/src/model.rs`
  - 新增 `MigrateSummary`
  - 输出字段包括：
    - `palace_path`
    - `sqlite_path`
    - `schema_version_before`
    - `schema_version_after`
    - `changed`
- `rust/src/service.rs`
  - 新增 `App::migrate()`
  - 它会：
    - 打开当前 palace SQLite
    - 读取迁移前版本
    - 调用 `init_schema()`
    - 返回迁移前后版本和是否发生变化
- `rust/src/main.rs`
  - 新增 `migrate` 子命令
- `rust/tests/cli_integration.rs`
  - 新增 `cli_migrate_upgrades_legacy_sqlite_schema`
  - 直接构造一个旧版 v1 SQLite palace，再通过 CLI 执行迁移
- `rust/README.md`
  - 文档加入 `migrate` 命令说明

# 关键知识

## 1. `migrate` 最重要的是“稳定输出”，不是“命令名存在”

如果命令只是执行成功/失败，没有结构化结果，后面很难被：

- shell 脚本
- 自动化 smoke test
- agent 工作流

稳定复用。

这次 `migrate` 返回 JSON，就是为了把它从“手工运维动作”变成“可编排接口”。

## 2. `changed` 字段很适合做幂等运维判断

迁移命令通常需要支持重复运行。  
如果每次都只返回“成功”，用户不知道到底有没有发生实际升级。

所以这次显式返回：

- `schema_version_before`
- `schema_version_after`
- `changed`

后面做脚本或 agent 判断时会更直接。

# 补充知识

## 为什么这里还是复用 `init_schema()` 而不是单独写一套 CLI 迁移代码

因为迁移逻辑应该只有一个权威入口。  
如果 CLI 里再自己写一套分支：

- service 一套
- CLI 一套

后面很容易出现行为漂移。  
这次做法是让 CLI 只是调 `App::migrate()`，而 `App::migrate()` 再复用存储层已有入口。

## 为什么这次还不做 `repair`

`migrate` 和 `repair` 看起来都属于运维命令，但问题性质不同：

- `migrate` 是已知版本之间的结构升级
- `repair` 往往意味着损坏恢复、索引重建、数据校验

后者风险更高，也更容易做过头。  
所以这里先把低风险、边界清晰的 `migrate` 做实，再做 `repair` 更稳。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增关键验证：

- `cli_migrate_upgrades_legacy_sqlite_schema`

这个测试会：

1. 手工创建一个 `schema_version = 1` 的旧 SQLite palace
2. 运行：
   - `cargo run -- --palace <palace> migrate`
3. 断言输出中包含：
   - `schema_version_before = 1`
   - `schema_version_after = 2`
   - `changed = true`

# 未覆盖项

这次没有继续做：

- `repair` CLI
- LanceDB 侧的独立迁移命令
- dry-run migrate
- 多步版本升级报告

所以这次提交的定位是：  
把 Rust 版已有的 schema 演进能力变成正式可调用的 CLI 命令，而不是继续停留在内部实现层。
