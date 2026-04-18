# 背景

前两轮 Rust `init` 已经补了两层 bootstrap：

1. 项目结构层：
   - `mempalace.yaml`
   - `entities.json`
2. AAAK / facts 文档层：
   - `aaak_entities.md`
   - `critical_facts.md`

但和 Python `entity_registry.py` 对照，还有一个明显缺口：

- `entity_registry.json`

这个文件不是给人直接读的，而是给后续 entity lookup / disambiguation / enrich 流程用的结构化持久层。没有它，Rust 这条线虽然已经有 bootstrap 文档，但还缺一个真正像“registry state”的文件。

所以这一提交的目标，是把 Rust `init` 再往 Python onboarding 靠一层：直接生成项目本地 `entity_registry.json`，并保持 local-first、非交互、可覆盖测试的路线。

# 主要目标

- 给 Rust `init` 增加 `entity_registry.json` bootstrap。
- 保持 registry schema 足够接近 Python 当前的持久层骨架。
- 如果文件已存在，保留而不覆盖。
- 把 registry path / written flag 接进 `InitSummary` 和 `init --human`。
- 把 `entity_registry.json` 排除出默认 project mining。

# 改动概览

- 更新 [rust/src/bootstrap.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/bootstrap.rs)
  - `InitBootstrap` 新增：
    - `entity_registry_path`
    - `entity_registry_written`
  - 新增：
    - `EntityRegistry`
    - `RegistryPerson`
    - `write_entity_registry()`
  - `bootstrap_project()` 现在会生成项目本地的 `entity_registry.json`
- 更新 [rust/src/model.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/model.rs)
  - `InitSummary` 同步新增 registry path / written flag
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - 让 init summary 带出 registry 信息
  - 并把 `entity_registry.json` 加入 project mining 默认 skip list
- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - `init --human` 新增：
    - `Registry: ...`
- 更新测试：
  - [rust/tests/service_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/service_integration.rs)
  - [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 1. `entity_registry.json` 和 `entities.json` 依然不是一回事

表面上看，它们都在存“哪些人/项目”。

但语义不同：

- `entities.json`
  - 更像轻量 bootstrap 列表
  - 适合直接给其它逻辑读取
- `entity_registry.json`
  - 更像长期状态容器
  - 会带：
    - `version`
    - `mode`
    - `people`
    - `projects`
    - `ambiguous_flags`
    - `wiki_cache`

也就是说，`entity_registry.json` 不是重复文件，而是“把 bootstrap 名单提升成 registry state”。

## 2. registry 最有价值的不是“能存人名”，而是未来能做 disambiguation

Python `entity_registry.py` 里真正有价值的部分，不只是把 Riley 存下来，而是后面能回答：

- 这个词是人名还是普通英文词？
- 它是 onboarding 来的，还是 wiki/research 来的？
- 有哪些 aliases？

Rust 这轮还没把完整 lookup / wiki / disambiguation 迁过来，但先把 registry schema 和 bootstrap 文件补齐，是必要的第一步。不然以后再加这些逻辑时，没有稳定的落盘面。

## 3. `ambiguous_flags` 值得提前写进 bootstrap

这轮顺手把一批常见“既像名字又像英文词”的候选放进了 `ambiguous_flags`，比如：

- `max`
- `may`
- `grace`

这样以后 Rust 真开始做 registry lookup 时，不需要再从零猜“哪些词需要上下文判别”，而是已经有一个 bootstrap 起点。

# 补充知识

## 1. project mining 必须跳过 bootstrap registry 文件，不然会自我污染

如果 `entity_registry.json` 不排除，后果会和前一轮 `aaak_entities.md` / `critical_facts.md` 一样：

- `mine` 会把 bootstrap 文件再当成项目内容吃进去
- `status` / `search` / `compress` / `wake-up` 会被这些元文件污染

所以这类“init 生成、后续逻辑再消费”的文件，默认都应该从普通 mining 里排掉。

## 2. local-first 路线下，把 registry 放项目目录比放 home 路径更利于多 palace 并行

Python 当前 registry 更偏全局 home 配置。

Rust 这里继续坚持 local-first：

- `project/entity_registry.json`

好处是：

- 多项目不会共用同一份 registry state
- 测试隔离更自然
- handoff / backup / zip 项目时更完整

这和前几轮把 `identity.txt`、`hook_state`、AAAK bootstrap 都本地化，其实是一条路线。

# 验证

本次实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo test --test service_integration init_project_bootstraps_rooms_and_entities
cargo test --test cli_integration cli_init_writes_entities_json_when_detection_finds_names
cargo test --test cli_integration cli_init_human_prints_python_style_summary
cargo check
```

提交前还会再跑一轮完整：

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这轮还没有迁 Python `entity_registry.py` 的 lookup / research / disambiguation 逻辑。
- `wiki_cache` 现在只是空壳位，没有接 Wikipedia 或其它研究面。
- registry 里的 `contexts/aliases/relationship` 仍是 bootstrap 默认值，没有 interactive onboarding 采集。
- 这轮没有新增 MCP surface；registry 目前只是 init 产物和后续能力底座。
