# 背景

前几轮我们一直在做同一件事：把 Rust 重写里原本挤在 `service.rs` 的领域逻辑，逐步送回更合适的模块。

已经拆出来的包括：

- `palace_graph`
- `knowledge_graph`
- `dedup`
- `repair`
- `layers`
- `searcher`
- `drawers`

继续往下看，`compress` 这条 AAAK 路径也还留着一块明显的 service 内嵌逻辑：

- 从 `DrawerRecord` 生成 `CompressedDrawer`
- 计算总 token 数
- 组装 `CompressSummary`

这些已经不只是“service 调一下 dialect”，而是一整块稳定的压缩规划逻辑。

# 主要目标

这一提交的目标是把 Rust 的 AAAK 压缩规划逻辑收成独立模块：

1. 新增 `compress` 模块
2. 把 `CompressedDrawer` 生成、token 汇总、summary 组装从 `service.rs` 挪出去
3. 保持 `compress` CLI / MCP 行为不变

# 改动概览

- 新增 [rust/src/compress.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/compress.rs)
  - `CompressionRun`
  - `CompressSummaryContext`
  - `CompressionRun::from_drawers()`
  - `CompressionRun::into_summary()`
  - 内部 `compressed_drawer_from_record()`
  - 补了 compression run 单测
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `compress` 模块
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `compress()` 现在只负责：
    - 取 drawers
    - 调 `CompressionRun::from_drawers()`
    - 按 dry-run 决定是否写 SQLite
    - 调 `into_summary()`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 补充 `compress` 模块说明

# 关键知识

## 1. “批处理规划” 很适合单独建模块

`compress` 这里的逻辑不是单条 drawer 处理，而是一批 drawers 的批处理：

- 每条 drawer 生成 AAAK
- 每条 drawer 算 compression stats
- 全局累计 token totals
- 最后再形成一次 summary

这类“批处理规划 + 汇总”特别适合抽成模块，因为它天然有自己的输入、输出和聚合规则。

## 2. service 更适合保留“是否落盘”的决策

这次没有把所有东西都搬走。`service.compress()` 仍然保留：

- `sqlite.list_drawers()`
- `sqlite.replace_compressed_drawers()`
- `dry_run` 的写入分支

因为这些是 orchestration。  
真正迁走的是：

- 压缩条目怎么生成
- totals 怎么累计
- summary 怎么拼

这能让边界更清楚，同时不把 storage 依赖一股脑塞进新模块。

# 补充知识

## 1. summary context 是压参数面最稳的办法

如果直接把 `palace_path/sqlite_path/version/wing/dry_run` 一长串传进 builder，很快就会碰到：

- 参数太多
- clippy 报 `too_many_arguments`
- 调用点很难读

这次用 `CompressSummaryContext` 把稳定上下文字段打包，和前面 `dedup`、`repair` 的做法保持一致，是一种很稳的 Rust 重构习惯。

## 2. 先抽“生成条目 + 统计 totals”，再考虑更大抽象

这次没有试图把 `Dialect`、SQLite、MCP、CLI 全都统一抽成一层 mega abstraction。  
只先把：

- `CompressedDrawer` 生成
- token 累计
- summary 组装

这三个稳定职责抽走。  
这是比较适合持续重写仓库的节奏：每次只抽一块成熟边界，不把系统一下子搞成“抽象比逻辑还多”。

# 验证

已运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有改变 AAAK dialect 本身的编码规则
- 这次没有改变 `compress` 的 CLI / MCP 参数和输出字段
- 这次没有改变 `wake_up` 的 Layer 1 生成逻辑
- 这次没有改 Python `dialect.py` / `cli.py`
- 这次也没有动 `hooks/`、`docs/`、`.github/` 或其它仓库
