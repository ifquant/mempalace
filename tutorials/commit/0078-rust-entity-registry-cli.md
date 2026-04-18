## 背景

前面 Rust `init` 已经能生成：

- `entities.json`
- `entity_registry.json`
- `aaak_entities.md`
- `critical_facts.md`

但当时 `entity_registry.json` 只是一个 bootstrap 产物。  
也就是说：

- 能写出来
- 但 Rust 自己并不会真正读取、查询、学习它

这和 Python 版 `entity_registry.py` 还有明显差距。  
Python 那边 registry 不是“静态文件”，而是一个真正参与行为决策的模块。

## 主要目标

这次要把 Rust 的 entity registry 从“写文件”推进到“可用能力”：

1. 抽成独立模块
2. 支持 load / save / summary / lookup / learn
3. 把 bootstrap 改成复用这个模块，而不是写第二份私有 schema
4. 提供 CLI：
   - `registry summary`
   - `registry lookup`
   - `registry learn`

## 改动概览

### 1. 新增独立模块 `rust/src/registry.rs`

这次新增了 [rust/src/registry.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/registry.rs)。

里面放了：

- `EntityRegistry`
- `RegistryPerson`
- `RegistryLookupResult`
- `RegistrySummary`
- `RegistryLearnSummaryFields`
- `SeedPerson`

核心能力：

- `load()`
- `save()`
- `seed()`
- `bootstrap()`
- `learn()`
- `lookup()`
- `summary()`

### 2. 把 bootstrap 的 registry schema 收回公共模块

之前 `bootstrap.rs` 里自己定义了一套：

- `EntityRegistry`
- `RegistryPerson`

这会导致两个问题：

1. schema 漂移风险高
2. 后续 service / CLI 还得再抄一次

现在改成：

- `bootstrap.rs` 直接复用 `registry.rs`
- `write_entity_registry()` 只负责“构造 + 保存”

这样 Rust 里只有一份 entity registry 真相来源。

### 3. service 层新增 registry 三个入口

在 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs) 新增：

- `registry_summary(&Path)`
- `registry_lookup(&Path, word, context)`
- `registry_learn(&Path)`

其中：

- `registry_summary`
  - 读取项目本地 `entity_registry.json`
  - 返回结构化统计和实体列表
- `registry_lookup`
  - 对一个词做 registry 查询
  - 支持上下文 disambiguation
- `registry_learn`
  - 复用现有 bootstrap 的本地 entity detector
  - 从项目文件里学习新 people / projects
  - 追加写回 `entity_registry.json`

### 4. CLI 新增 `registry`

在 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs) 新增了：

- `registry summary <dir>`
- `registry lookup <dir> <word> --context "..."`
- `registry learn <dir>`

而且继续遵守 Rust 版 CLI 的双输出原则：

- 默认 JSON
- `--human` 输出人类可读摘要

这点很重要，因为这让 registry 能同时服务：

- 脚本
- 自动化
- 人工排查

### 5. `lookup()` 已经有 Python 风格歧义消解

这次没有只做“精确命中”。

Rust `lookup()` 也迁了 Python 里的关键一层：

- 如果某个名字同时也是常见英文词，比如 `Ever`
- 且上下文里出现了人名模式：
  - `Ever said ...`
- 就判成 `person`
- 如果上下文更像普通概念：
  - `Have you ever ...`
- 就判成 `concept`

这让 registry 不再只是一个简单字典。

## 关键知识

### 1. “把 bootstrap 文件写出来”和“把 registry 变成能力”是两回事

很多系统做到：

- 初始化时生成配置

就停了。  
但真正可替代 Python 的 Rust 版需要继续做到：

- 后续命令可以读它
- 后续命令可以查它
- 后续命令可以增量学习它

否则这个文件只是“摆设”。

### 2. 歧义词的人名判断不能只靠词典

像：

- `Ever`
- `Grace`
- `Will`

这些词如果只看 registry，很容易误判。  
所以 Rust 这次保留了 Python 的思路：

- registry 给出“这个词可能是人名”
- context pattern 再决定当前句子里它到底是不是人名

这比“所有命中都当 person”稳很多。

## 补充知识

### 1. 为什么 `registry learn` 要复用 bootstrap 的 detector

因为 Rust 里已经有一套本地 entity detection 逻辑在 `bootstrap.rs`：

- `scan_for_detection`
- `detect_entities`
- 人名/项目打分

如果再为 registry learn 造第二套：

- 规则会分叉
- 结果会不一致
- 测试面会翻倍

所以这次做法是把 bootstrap 探测结果复用给 registry learn。

### 2. 为什么这次没有迁 Python 的 Wikipedia research

Python `entity_registry.py` 还有：

- wiki lookup
- confirm research

但这轮先没做，原因很实际：

- 本地 lookup / summary / learn 已经能明显提升 Rust 的可用性
- 而 wiki research 会引入网络路径、缓存确认流程、以及更多交互表面

先把本地 registry 主链路做实，比半套 online research 更值。

## 验证

已实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

本轮新增覆盖重点：

- `registry::tests::lookup_disambiguates_ambiguous_names_with_context`
- `registry::tests::registry_load_save_round_trip`
- `registry_summary_lookup_and_learn_work`
- `cli_registry_help_mentions_summary_lookup_and_learn`
- `cli_registry_summary_lookup_and_learn_work`

## 未覆盖项

- 还没有迁 Python `entity_registry.py` 的 Wikipedia research / confirm 流程
- 还没有单独做 alias 编辑/管理 CLI
- 还没有把 registry 能力接进 MCP
- 还没有做完整 interactive onboarding CLI；这轮主要补的是 registry 主体能力，而不是问答式 first-run 流程
