## 背景

前一轮已经把 Rust 的 layer 能力接进了 MCP，但 palace 维护面还留在 CLI：

- `repair`
- `repair scan`
- `repair prune`
- `repair rebuild`
- `compress`
- `dedup`

这会造成一个实际问题：终端里可以做维护和 AAAK 压缩，MCP host 却看不到这些能力，自动化代理只能停在读面。

## 主要目标

- 给 Rust MCP 新增 `mempalace_repair`
- 给 Rust MCP 新增 `mempalace_repair_scan`
- 给 Rust MCP 新增 `mempalace_repair_prune`
- 给 Rust MCP 新增 `mempalace_repair_rebuild`
- 给 Rust MCP 新增 `mempalace_compress`
- 给 Rust MCP 新增 `mempalace_dedup`

## 改动概览

- 在 `rust/src/mcp.rs` 的 `tools()` 里补了 6 个 maintenance / AAAK 工具
- 在 `call_tool()` 里把这 6 个工具直接接到现有 service：
  - `App::repair()`
  - `App::repair_scan()`
  - `App::repair_prune()`
  - `App::repair_rebuild()`
  - `App::compress()`
  - `App::dedup()`
- 在 `coerce_argument_types()` 里补了：
  - `confirm`
  - `dry_run`
  - `threshold`
  - `stats_only`
  - `min_count`
- 在 `rust/tests/mcp_integration.rs` 里把成功路径回归一次锁住
- 在 `rust/README.md` 里把这组 MCP maintenance tools 写成仓库事实

## 关键知识

这次仍然坚持一个原则：**MCP 只暴露 service，尽量不在 MCP 层重写业务逻辑**。

例如：

- `mempalace_repair_scan` 不自己去比对 SQLite / LanceDB
- `mempalace_repair_rebuild` 不自己去管 re-embed
- `mempalace_dedup` 不自己去算 cosine distance

而是全部走已有 `App::*` 方法，再把 summary 直接序列化成 JSON。

这样做的好处是：

1. CLI / MCP 共享同一套业务行为
2. 以后修 drift / dedup / compress 逻辑时，只改 service 就够了
3. 集成测试可以专门锁“工具面是否暴露正确”，而不是重复覆盖算法细节

## 补充知识

### 1. MCP 最容易分叉的地方不是算法，而是默认值

像 `dedup` 里的默认值：

- `threshold = 0.15`
- `min_count = 5`
- `dry_run = false`

如果 MCP 自己发明一套默认值，CLI 和 MCP 很快就会看起来“都能跑”，但实际行为不一致。  
所以这次 MCP 明确沿用了 CLI 同一组默认值。

### 2. `repair_prune` 这种工具最好把 preview 和 destructive mode 共用一个入口

`confirm = false` 时只是 preview，`confirm = true` 时才真的删。  
这比拆成两个不同工具更稳，因为：

- host 侧更容易发现“先 preview，再 confirm”这条路径
- 后面加字段时不会有两套 schema 漂移

## 验证

实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

本次新增回归至少覆盖了：

- `tools/list` 里出现 maintenance / AAAK MCP 工具
- `mempalace_repair` 返回健康摘要
- `mempalace_repair_scan` 返回 drift scan 结果
- `mempalace_repair_prune` 在 `confirm=false` 下返回 preview
- `mempalace_compress` 在 `dry_run=true` 下返回压缩摘要
- `mempalace_dedup` 返回 dedup 统计摘要
- `mempalace_repair_rebuild` 返回重建结果

## 未覆盖项

- 这次没有补 broken-sqlite 或 vector drift 的 MCP 专项失败回归；当前仍主要依赖已有工具级 `error + hint` 通道
- 这次没有把 `normalize`、`split`、`onboarding` 这些高层 CLI 也接进 MCP
- 这次没有新增 Python 端对应 MCP 工具，只扩了 Rust MCP surface
