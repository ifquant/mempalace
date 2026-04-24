# 背景

前一轮我们已经把 Rust 里的 `dedup` 逻辑从 `service.rs` 拆成了单独模块。接着往下看，`repair` 这条线也还有一块明显的“领域逻辑堆在 service 里”的问题：

- `repair()` 负责拼 diagnostics summary
- `repair_scan()` 负责比对 SQLite / LanceDB ID，并写 `corrupt_ids.txt`
- `repair_prune()` 负责读取 `corrupt_ids.txt` 并拼结果
- `repair_rebuild()` 负责做本地 SQLite backup，并拼 rebuild summary

这些逻辑并不都是“服务编排”。其中不少其实更像 Python `repair.py` 那种独立的 maintenance helper，只是暂时被塞进了 Rust 的 `service.rs`。

# 主要目标

这一提交的目标是把 Rust 的 repair 领域逻辑再收紧一层：

1. 新增独立 `repair` 模块，对齐 Python `repair.py` 的模块边界
2. 把 `repair scan / prune / rebuild` 的 summary 组装和文件辅助逻辑从 `service.rs` 挪出去
3. 保持 CLI、MCP、测试和用户可见输出完全不变

# 改动概览

- 新增 [rust/src/repair.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/repair.rs)
  - `RepairContext`
  - `RepairContext::build_summary()`
  - `RepairContext::build_scan_summary()`
  - `RepairContext::build_prune_preview()`
  - `RepairContext::build_prune_result()`
  - `RepairContext::build_rebuild_summary()`
  - `read_corrupt_ids()`
  - `backup_sqlite_source()`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出新模块 `repair`
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `repair()` 改为委托 `RepairContext::build_summary()`
  - `repair_scan()` 改为委托 `RepairContext::build_scan_summary()`
  - `repair_prune()` 改为复用 `read_corrupt_ids()` 和 `RepairContext` 的 summary builder
  - `repair_rebuild()` 改为复用 `backup_sqlite_source()` 和 `RepairContext::build_rebuild_summary()`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 补充 Rust `repair` 模块说明

# 关键知识

## 1. service 层不一定要“什么都自己做”

`service.rs` 更适合保留：

- 打开 SQLite / LanceDB
- 调 embedder
- 控制执行顺序
- 把多个底层组件串起来

但像下面这些就更适合下沉到领域 helper：

- 如何从 drawer 集合生成 repair scan summary
- 如何从 `corrupt_ids.txt` 读取 queued IDs
- 如何统一拼 `repair_prune` / `repair_rebuild` 的结果结构

这样做的好处是：service 的职责更清晰，后面如果 MCP、CLI、库层都要复用 repair 逻辑，也更容易保持一致。

## 2. `RepairContext` 适合装“稳定上下文”

这次没有让每个 builder 都吃一长串参数，而是引入了 `RepairContext`：

- `palace_path`
- `sqlite_path`
- `lance_path`
- `version`

这些字段在一轮 repair 操作里基本是稳定的。把它们打包进 context，有两个好处：

1. builder 的参数不会越来越长
2. repair 的多个 summary 更容易保持同一套路径/version shape

这和前一轮 `dedup` 里用 context 压缩 summary builder 参数是同一个思路。

# 补充知识

## 1. “读文件 helper” 往往是最容易抽出的第一步

像 `read_corrupt_ids()` 这种函数很适合先抽，因为它：

- 输入输出明确
- 没有异步依赖
- 不直接碰业务 orchestrator
- 单测成本很低

新人做 Rust 模块化时，如果不知道从哪里开始，先找这种纯函数 / 小 IO helper，通常最稳。

## 2. summary builder 和执行逻辑分离，能降低回归面

这次没有改 repair 的外部行为，只是把“怎么拼结果”独立出来。这个做法的好处是：

- CLI 测试大概率不用改
- MCP 测试大概率不用改
- 真正高风险的行为路径没变

对于持续重构中的重写项目，这是很实用的策略：先拆边界，再考虑扩功能。

# 验证

已运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有改变 `repair` CLI / MCP 的任何用户可见参数或输出
- 这次没有把 rebuild 的 embedding loop 从 `service.rs` 挪走，因为它仍然强依赖当前 `embedder + vector store` 编排
- 这次没有改 Python `repair.py`
- 这次也没有动 `hooks/`、`docs/`、`.github/` 或其它仓库
