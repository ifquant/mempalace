## 背景

上一轮 Rust 已经有了：

- `registry summary`
- `registry lookup`
- `registry learn`

这意味着 `entity_registry.json` 不再只是 bootstrap 文件。  
但它还是缺一层很关键的“日常维护面”：

- 手工加人
- 手工加项目
- 加 alias / nickname
- 对一条查询做 registry-aware 解析

如果没有这层能力，用户还是得回去手改 JSON，或者依赖 `learn` 被动追加。

## 主要目标

这次要把 Rust registry 再推进一层，做到：

1. 能手工维护 registry
2. 能把 alias 正常连到 canonical person
3. 能从自然语言 query 里抽取：
   - 已知 people
   - 仍未知但像实体的 capitalized candidates

对应 CLI：

- `registry add-person`
- `registry add-project`
- `registry add-alias`
- `registry query`

## 改动概览

### 1. registry.rs 补了写面和 query helper

在 [rust/src/registry.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/registry.rs) 新增：

- `add_person()`
- `add_project()`
- `add_alias()`
- `extract_people_from_query()`
- `extract_unknown_candidates()`

这让 registry 本体不再只是：

- load/save/lookup/learn

而是具备了“维护 + 检索辅助”两类实际能力。

### 2. alias 现在会建立 canonical 回指

`add_alias()` 做的不是简单往原人名上塞字符串，而是两层同步：

1. canonical person 的 `aliases` 里追加 alias
2. 同时在 registry 里写一个 alias entry：
   - `canonical = <主名字>`
   - 其它 metadata 继承主条目

这样后面：

- `lookup()`
- `extract_people_from_query()`

都能把 alias 正常收敛到 canonical 人名，而不是把 alias 当成第二个人。

### 3. query helper 开始有“检索前理解”能力

新增的 `extract_people_from_query()`：

- 会检查 canonical name
- 也会检查 alias
- 对歧义词仍复用上下文判别
- 最终统一返回 canonical name

新增的 `extract_unknown_candidates()`：

- 会抓取 query 里的 capitalized words
- 跳过常见英文词
- 跳过 registry 已知实体
- 返回仍未知的候选实体

这和 Python `entity_registry.py` 里的查询辅助面是同方向的。

### 4. service 层补了 registry 写/查 API

在 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs) 新增：

- `registry_add_person()`
- `registry_add_project()`
- `registry_add_alias()`
- `registry_query()`

这样 CLI 不需要直接操作 registry 文件，后面如果要接 MCP，也能继续复用 service 层而不是重复逻辑。

### 5. CLI 新增 4 个 registry 子命令

在 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs) 新增：

- `registry add-person <dir> <name> --relationship ... --context ...`
- `registry add-project <dir> <name>`
- `registry add-alias <dir> <canonical> <alias>`
- `registry query <dir> "<text>"`

并且继续保持：

- 默认 JSON
- `--human` 人类输出

## 关键知识

### 1. alias 的正确做法不是“字符串替换”，而是 canonical normalization

如果只把 alias 存成一个普通字符串列表，但查询时又把 alias entry 当作独立 person，
就会出现：

- `Jordan`
- `Jordy`

被当成两个不同实体。

这次修的关键点就是：

- query helper 最终必须收敛到 canonical name

所以测试里特意锁住：

- `Jordy said ...`
  - 只能返回 `Jordan`
  - 不能返回 `Jordan` 和 `Jordy` 两份

### 2. “unknown candidates” 很适合放在 registry 层，而不是 search 层

因为这件事本质不是向量检索，而是：

- 先做实体认知
- 再决定是不是要 research / confirm / add

把这一步放到 registry，更符合职责边界。

## 补充知识

### 1. 为什么这轮没有直接做 Wikipedia research

Python 还有：

- `research()`
- `confirm_research()`

但这轮先没接，原因是：

- 先把本地 registry 的“维护面 + 查询面”做实
- 再接 online research 更稳

否则会变成：

- 在线查询能跑
- 但本地 alias / add-person / query 反而还不完整

### 2. 为什么 `registry query` 返回“已知 people + unknown candidates”

这是故意拆成两类结果：

- `people`
  - 已经确认过的实体
- `unknown_candidates`
  - 可能值得后续 research / add 的名字

这样后续不管是：

- CLI
- MCP
- agent 自动化

都可以把“识别已知对象”和“发现未知对象”分开处理。

## 验证

已实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

本轮新增覆盖重点：

- `registry::tests::registry_extracts_people_and_unknown_candidates_from_query`
- `cli_registry_help_mentions_summary_lookup_and_learn`
- 继续复用前一轮 `cli_registry_summary_lookup_and_learn_work`

## 未覆盖项

- 还没有迁 Python 的 Wikipedia `research` / `confirm_research`
- 还没有做 registry 删除/重命名命令
- 还没有把 registry query 接到 MCP
- 还没有做 interactive onboarding 问答流程；当前仍以 `init` bootstrap + registry CLI 维护为主
