## 背景

Rust 版 MemPalace 之前已经有：

- `repair` 诊断
- `migrate`
- `status`
- `compress` / `wake-up`

但“运维闭环”还差一整块。Python 侧除了只读诊断，还有两类很实际的能力：

1. `repair.py` 的 `scan / prune / rebuild`
2. `dedup.py` 的重复 drawer 清理

如果 Rust 只会报告问题，不会真正修，就还不能算把高层 CLI 收到位。

## 主要目标

这次要把 Rust 的运维面往 Python 再收一大块：

- 保留原来的 `repair` 诊断入口
- 新增：
  - `repair scan`
  - `repair prune --confirm`
  - `repair rebuild`
- 新增：
  - `dedup`

同时要求这几条不是空壳：

- 要有真实 SQLite + LanceDB 行为
- 要有 CLI 回归
- 要有 service 回归
- 要继续维持 JSON / `--human` 双输出风格

## 改动概览

### 1. 扩了 repair 的结构化结果

在 `rust/src/model.rs` 新增了：

- `RepairScanSummary`
- `RepairPruneSummary`
- `RepairRebuildSummary`
- `DedupSummary`
- `DedupSourceResult`

这样 CLI、service、以后 MCP 或别的 surface 都可以共享同一套结果结构，而不是每层自己拼 JSON。

### 2. 给 LanceDB 补了“运维需要的底层接口”

在 `rust/src/storage/vector.rs` 新增了：

- `delete_drawers()`
- `clear_table()`
- `list_drawers()`

并补了 `VectorDrawer`，能把 LanceDB 里的：

- `id`
- `wing`
- `room`
- `source_file`
- `source_path`
- `text`
- `vector`

读回 Rust，用于：

- repair scan 比对 SQLite / LanceDB ID 漂移
- dedup 在同一 source group 内做向量距离判重

### 3. repair 现在不只是诊断

在 `rust/src/service.rs` 里新增了：

- `repair_scan()`
- `repair_prune()`
- `repair_rebuild()`

具体行为：

- `repair`
  - 仍保留旧的非破坏性诊断
- `repair scan`
  - 对比 SQLite drawer IDs 和 LanceDB drawer IDs
  - 区分：
    - `missing_from_vector`
    - `orphaned_in_vector`
  - 把可 prune 的 orphan IDs 写到 `<palace>/corrupt_ids.txt`
- `repair prune --confirm`
  - 读取 `corrupt_ids.txt`
  - 删除对应 LanceDB / SQLite 记录
  - 不加 `--confirm` 时只做 dry-run
- `repair rebuild`
  - 以 SQLite 作为 source of truth
  - 重新 embed drawers
  - 清空 LanceDB table
  - 重新写回全部向量
  - 同时备份 `palace.sqlite3`

这里最关键的设计点是：

- Rust 的 source of truth 是 SQLite
- 所以 rebuild 不能像 Python 一样只靠“再建 HNSW”
- 必须从 SQLite 重新生成向量层

### 4. 新增 dedup CLI 和 service

在 `rust/src/service.rs` 里新增了 `dedup()`，在 `rust/src/main.rs` 新增了 `dedup` 命令。

逻辑基本贴 Python：

- 先按 `source_file` 分组
- 小于 `min_count` 的组不检查
- 同组内按文本长度优先保留“更丰富”的 drawer
- 用向量 cosine distance 和已保留 drawer 比较
- 低于阈值就判成重复

支持：

- `--threshold`
- `--dry-run`
- `--stats`
- `--wing`
- `--source`
- `--human`

### 5. CLI 帮助和人类输出也一起跟上

在 `rust/src/main.rs` 里：

- `repair` 从单一命令变成“默认诊断 + 可选子命令”
- 新增 `RepairCommand`
- 增加：
  - `print_repair_scan_human()`
  - `print_repair_prune_human()`
  - `print_repair_rebuild_human()`
  - `print_dedup_human()`

这样：

- 默认仍然输出结构化 JSON
- `--human` 时会给出运维友好的文字摘要

## 关键知识

### 1. Rust 版 repair 和 Python 版 repair 的 source of truth 不一样

Python 旧架构里，ChromaDB collection 本身就是主要真相来源。  
Rust 版现在是：

- SQLite 保存 drawer 元数据和正文
- LanceDB 保存向量检索层

所以 repair 的语义必须跟着架构改：

- `scan` 主要看 SQLite / LanceDB 是否漂移
- `prune` 主要删 vector orphan
- `rebuild` 主要从 SQLite 重建 LanceDB

这不是“与 Python 不一致”，而是“在 Rust 架构下复刻 Python 的目的”。

### 2. dedup 的真正难点不是删，而是“决定保留谁”

这次保留了 Python 的核心策略：

- 同一 `source_file` 内比较
- 优先保留更长、信息量更高的 drawer

这比“看见重复就删后来的”稳得多，因为多次重挖时往往新版 chunk 更完整。

## 补充知识

### 1. 为什么 `repair scan` 只把 orphan IDs 写到 `corrupt_ids.txt`

因为：

- `missing_from_vector`
  - 说明 SQLite 里还有记录
  - 这种情况更适合 `rebuild`
- `orphaned_in_vector`
  - 说明 LanceDB 里有 SQLite 不认识的垃圾
  - 这种情况才适合 `prune`

如果把两类都塞进 prune 队列，就会误删 SQLite 真正保留的数据。

### 2. 为什么 `repair rebuild` 要重新 embed

因为当前 SQLite 没有存原始 embedding。  
所以 rebuild LanceDB 的唯一办法是：

1. 从 SQLite 读 text
2. 重新走 embedder
3. 重新写 LanceDB

这也意味着：

- rebuild 依赖当前 embedder profile
- 如果 embedding profile 不匹配，repair 本身就应该先失败，而不是偷偷写错维度的数据

## 验证

已实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
```

这轮新增覆盖的重点包括：

- `repair_scan_prune_and_rebuild_handle_vector_drift`
- `dedup_removes_near_identical_drawers_from_same_source`
- `cli_repair_scan_and_rebuild_cover_vector_drift`
- `cli_dedup_human_prints_summary`
- `cli_dedup_help_mentions_threshold_and_stats`

## 未覆盖项

- 还没有把 Python `repair.py` 的 `wing` 范围 prune / rebuild 全部照搬到 Rust
- 还没有做更激进的 vector-level corruption 检测，只做了 SQLite / LanceDB 漂移检查
- `dedup` 目前仍然是 CLI / service 面，没有额外扩到 MCP
- 还没有做 dedup 的 stats-only 更细粒度输出，例如 top-N source groups 的单独结构化明细
