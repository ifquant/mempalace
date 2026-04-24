# 背景

上一提交已经把 `migrate` 做成了正式 CLI，Rust 版现在有了：

- 内部 schema 演进能力
- 外部可调用的迁移命令

但真实运维里还有另一类高频需求：

- 这个 palace 现在健康吗？
- SQLite 在不在？
- LanceDB 能不能打开？
- embedding profile 有没有对上？

这类问题还不适合直接做“修复动作”，因为修复通常意味着：

- 重建索引
- 改写数据
- 删除旧状态

风险太高。

# 主要目标

这次提交的目标是先做一个低风险 `repair` 入口，但它当前只负责：

1. 诊断
2. 一致性检查
3. 结构化输出

不负责重建和删除。

# 改动概览

主要改动如下：

- `rust/src/model.rs`
  - 新增 `RepairSummary`
  - 输出内容包括：
    - palace / sqlite / lance 路径
    - 路径是否存在
    - `schema_version`
    - SQLite drawer 数
    - embedding profile
    - LanceDB 是否可访问
    - `ok`
    - `issues`
- `rust/src/service.rs`
  - 新增 `App::repair()`
  - 检查内容包括：
    - SQLite 文件是否存在
    - LanceDB 目录是否存在
    - SQLite schema 是否可初始化
    - embedding profile 是否和当前 provider 匹配
    - LanceDB table 是否可打开/确保存在
- `rust/src/main.rs`
  - 新增 `repair` 子命令
- `rust/tests/cli_integration.rs`
  - 新增：
    - `cli_repair_reports_missing_palace_non_destructively`
    - `cli_repair_reports_healthy_hash_palace`
- `rust/README.md`
  - 补充 `repair` 命令说明和当前边界

# 关键知识

## 1. “repair” 不一定一开始就要真的修

很多系统一做 repair，就直接上：

- rebuild
- rewrite
- delete-and-recreate

但这对重写阶段很危险。  
因为你还没把错误场景摸清楚，就先给了自己一个可能破坏数据的入口。

所以这次先把 `repair` 做成：

- 只读诊断
- 非破坏性检查
- 结构化报告

这更适合当前阶段。

## 2. `ok + issues[]` 比单一布尔值更适合运维接口

只返回 `ok: true/false` 的问题是：

- 你知道坏了
- 但不知道坏在哪

所以这次 `repair` 同时返回：

- `ok`
- `issues`

这样既适合脚本快速判断，也适合人类和 agent 继续往下分析。

# 补充知识

## 为什么这里会检查 embedding profile

因为对 Rust 版来说，一个很常见的真实问题不是“数据库坏了”，而是：

- 当前 palace 是 `hash`
- 但你现在用 `fastembed` 打开

这时候如果没有明确诊断，很容易被误解成：

- LanceDB 坏了
- 搜索坏了
- schema 坏了

所以 repair 里把 profile mismatch 单独报出来，信息价值很高。

## 为什么 `repair` 允许在 LanceDB 目录存在时调用 `ensure_table`

因为这次 repair 的目标是验证“当前向量层是否可访问”，而不是严格只读取元数据。  
`ensure_table()` 在已有 palace 上是低风险动作：

- 不删除数据
- 不重建索引
- 只是在需要时补空表

在当前阶段，这个折中比引入一套更复杂的只读元数据探测更实际。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增关键验证：

- `cli_repair_reports_missing_palace_non_destructively`
  - 验证不存在 palace 时会返回 `ok: false` 和缺失项
- `cli_repair_reports_healthy_hash_palace`
  - 验证一个正常的 hash palace 会返回：
    - `ok: true`
    - `vector_accessible: true`
    - `embedding_provider: "hash"`
    - `schema_version: 2`

# 未覆盖项

这次没有继续做：

- 真正的索引重建
- SQLite / LanceDB 数据修复
- dry-run / fix 模式分离
- 损坏 palace 的自动恢复

所以这次提交的定位是：  
先把 `repair` 做成一个低风险的正式诊断入口，为以后真正的修复能力留出清晰边界。
