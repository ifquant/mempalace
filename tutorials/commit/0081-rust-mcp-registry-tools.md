## 背景

上一轮已经把 Rust 的 entity registry 扩成了完整 CLI：

- `summary`
- `lookup`
- `learn`
- `add-person`
- `add-project`
- `add-alias`
- `query`
- `research`
- `confirm`

但这些能力还只停在 CLI。Python 版的方向已经很明确：MCP 要成为 AI 实际工作时的主入口，所以 registry 这层如果不进 MCP，就仍然是半截对齐。

## 主要目标

- 把 project-local registry 读写能力接进 Rust MCP
- 保持 Python 风格：
  - 工具级 `error + hint`
  - 缺参不抬 transport error
  - 写工具继续进入 WAL 审计
- 覆盖 read / write / research / confirm，而不是只补 summary

## 改动概览

- 在 [rust/src/mcp.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/mcp.rs) 新增 MCP 工具：
  - `mempalace_registry_summary`
  - `mempalace_registry_lookup`
  - `mempalace_registry_query`
  - `mempalace_registry_learn`
  - `mempalace_registry_add_person`
  - `mempalace_registry_add_project`
  - `mempalace_registry_add_alias`
  - `mempalace_registry_research`
  - `mempalace_registry_confirm`
- 同时补齐：
  - `tools/list`
  - `tools/call`
  - `requires_existing_palace()`
  - `coerce_argument_types()`
  - registry 写操作的 WAL
- 在 [rust/tests/mcp_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/mcp_integration.rs) 新增 registry MCP 成功路径和缺参错误路径回归
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. registry MCP 不应该依赖 palace 已存在

这轮最容易犯的错，是把 registry tools 也塞进 “必须已有 palace” 那条判断里。

但 registry 是 **project-local**：

- 目标文件是 `project_dir/entity_registry.json`
- 它和 palace 的 `palace.sqlite3` / `lance/` 不是同一层资源

所以这轮特意把 registry MCP tools 放进 `requires_existing_palace()` 的豁免名单里。  
否则 AI 明明只是在问项目里的 entity registry，却会被错误拦截成 `No palace found`。

### 2. MCP 写工具仍然要进 WAL

前面已经把 drawer / KG / diary 写面接进了 palace-local `wal/write_log.jsonl`。  
registry 这层虽然不是 palace 存储本体，但从 AI 协作角度看，它同样属于“会改变系统记忆边界”的写操作。

所以这轮继续保持一致性：

- `registry_add_person`
- `registry_add_project`
- `registry_add_alias`
- `registry_research`
- `registry_confirm`

都会进入 WAL。

这能保证以后排查 “是谁把某个名字确认成 person” 时，不会只剩 CLI 历史。

### 3. research 测试仍然避免真实联网

虽然 MCP 现在已经能触发 `registry_research`，但测试里仍然不直接依赖 Wikipedia。

方式和 CLI 那轮一致：

- 先往 `entity_registry.json` 里写入预置 `wiki_cache`
- MCP `registry_research` 命中缓存
- 再通过 `registry_confirm` 验证推进逻辑

这样：

- 功能面是真正可用的
- 测试面不被外网波动拖垮

## 补充知识

### 为什么 registry tools 的参数里直接用 `project_dir`

这里没有额外设计 project handle / registry id，而是直接要求 `project_dir`，原因是当前 Python / Rust 两边都还没有一个更高层的 “多项目 registry locator”。

直接传 `project_dir` 的好处：

- 行为明确
- 本地优先
- 和 CLI / service 层路径模型一致

等以后真的有多项目管理层，再统一抽象也不迟。

### 为什么 read/write/research/confirm 一次全补

如果只补 `summary/lookup/query`，AI 真正工作时仍然会在“发现 unknown candidate 后该怎么办”这个地方断掉。

所以这轮故意不是小碎片，而是把 registry MCP 一整组收口：

- 读
- 学
- 写
- 研究
- 确认

这样接下来无论是 onboarding agent，还是对话期的 memory agent，都能直接走 MCP 闭环。

## 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 还没有把 registry MCP 写面单独分到专门的 audit topic 或更细粒度 op code
- 还没有把 registry 能力接进 interactive onboarding agent
- 还没有做 registry delete / rename / merge 的 MCP surface
- 还没有做 registry research 的 richer source provider（现在还是 Wikipedia 路线）
