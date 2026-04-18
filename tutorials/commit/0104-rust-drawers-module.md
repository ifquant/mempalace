# 背景

前几轮我们一直在做同一类收口：把 Rust 重写里原本堆在 `service.rs` 的领域逻辑，逐步送回各自更合适的模块。

已经拆出来的包括：

- `palace_graph`
- `knowledge_graph`
- `dedup`
- `repair`
- `layers`
- `searcher`

继续往下看，`add_drawer()` 这条路径里还有一组很明确的小边界：

- 手工 drawer 的 ID 生成
- `wing/room/added_by` 名称校验
- `content` 清洗
- `DrawerRecord -> DrawerInput` 转换

这些逻辑在 Python 里分散出现在 `miner.py` 和 `mcp_server.py`，在 Rust 里继续留在 `service.rs` 已经不太合适了。

# 主要目标

这一提交的目标是把 Rust 的 drawer 写入辅助逻辑收成独立模块：

1. 新增 `drawers` 模块
2. 把 manual drawer 构造、ID 生成、名称清洗、record 转 input 迁出 `service.rs`
3. 保持 `add_drawer`、rebuild 路径、CLI/MCP 行为不变

# 改动概览

- 新增 [rust/src/drawers.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/drawers.rs)
  - `build_manual_drawer()`
  - `drawer_input_from_record()`
  - `sanitize_name()`
  - 内部 `sanitize_content()`
  - 内部 `identifier_fragment()`
  - 补了 manual drawer 相关单测
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `drawers` 模块
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `add_drawer()` 现在直接复用 `build_manual_drawer()`
  - rebuild 继续复用 `drawer_input_from_record()`
  - 删除 service 内部那组 drawer helper
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 补充 `drawers` 模块说明

# 关键知识

## 1. “构造输入对象” 也是值得单独建模块的逻辑

很多时候大家会把这种代码误判成“只是几行 glue code”，但这里其实包含了完整规则：

- drawer_id 怎么生成
- `source_path` 为空时怎么退回 `mcp://...`
- `ingest_mode` / `extract_mode` 怎么设
- 哪些字段需要 sanitize

这已经不只是机械赋值了，而是一套写入协议。  
把它独立出来之后，service 就不需要再自己知道这些细节。

## 2. Rust 这次对齐的是 Python 的“写入规则”，不是只对齐字段名

Python 里手工 add drawer 的 ID 规则是：

- `drawer_<wing>_<room>_<hash>`

Rust 之前虽然表面行为一致，但那套规则还内嵌在 service 里。  
这次把它收进 `drawers` 模块后，等于把这条协议真正变成了可复用、可测试的库层能力。

# 补充知识

## 1. 提炼“稳定协议”时，优先抽可复用构造器

`build_manual_drawer()` 这种函数很适合作为一个模块切片，因为它：

- 输入明确
- 输出明确
- 不依赖异步上下文
- 可直接做单测

这类函数通常是从大 service 里拆边界最稳的一步。

## 2. rebuild 路径和 MCP/manual 写入路径共用转换器，很值

`drawer_input_from_record()` 看起来很简单，但它把两条原本不同来源的路径接到了同一个输入形状：

- 从 SQLite 读回来的 drawer
- 手工写入前的 DrawerInput

这种“统一输入形状”的 helper 会明显减少以后 schema 演进时漏改字段的概率。

# 验证

已运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次没有改变 `add_drawer` / `delete_drawer` 的 CLI 或 MCP 参数
- 这次没有改变 drawer 的持久化 schema
- 这次没有改 Python `miner.py` 或 `mcp_server.py`
- 这次也没有动 `hooks/`、`docs/`、`.github/` 或其它仓库
