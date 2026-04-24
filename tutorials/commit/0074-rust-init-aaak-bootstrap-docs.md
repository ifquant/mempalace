# 背景

上一轮 Rust `init` 已经不只是“建 palace”，还会在项目目录里做基础 bootstrap：

- 生成 `mempalace.yaml`
- 在有足够信号时生成 `entities.json`

但和 Python `onboarding.py` 对照，还差一个很实用的高层产物：

- `aaak_entities.md`
- `critical_facts.md`

这两个文件的作用不是索引原文，而是把“这个项目世界里有哪些人、有哪些项目、初始 palace 长什么样”提前写成一个紧凑 bootstrap 层。这样后面：

- `compress`
- `wake-up`
- 人工校对实体
- 后续 agent 接手

都会轻很多。

所以这一提交的目标，是把 Python onboarding 里的 AAAK bootstrap 文档能力，按 Rust 当前 local-first 路线收成一个非交互、项目本地、可测试的 `init` 扩展。

# 主要目标

- 给 Rust `init` 增加 `aaak_entities.md` 生成。
- 给 Rust `init` 增加 `critical_facts.md` 生成。
- 保持和前一轮一样的“已有文件不覆盖”策略。
- 把这两个产物接进 `InitSummary` 和 `init --human`。
- 补齐 service / CLI 回归。

# 改动概览

- 更新 [rust/src/bootstrap.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/bootstrap.rs)
  - `InitBootstrap` 新增：
    - `aaak_entities_path`
    - `aaak_entities_written`
    - `critical_facts_path`
    - `critical_facts_written`
  - 新增：
    - `write_aaak_entities()`
    - `write_critical_facts()`
    - `entity_code()`
  - `bootstrap_project()` 现在除了 `mempalace.yaml` / `entities.json`，还会生成：
    - `aaak_entities.md`
    - `critical_facts.md`
- 更新 [rust/src/model.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/model.rs)
  - `InitSummary` 同步补上这四个字段
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `init_project()` 把新的 bootstrap 路径和 written flags 填进 summary
  - 普通 `init()` 继续返回这些字段为 `None/false`
- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - `init --human` 现在会显示：
    - `AAAK: ...`
    - `Facts: ...`
- 更新测试：
  - [rust/tests/service_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/service_integration.rs)
  - [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 1. `aaak_entities.md` 和 `entities.json` 不是重复文件，它们服务不同读者

这两个文件看起来都在表达“有哪些人/项目”，但用途不一样：

- `entities.json`
  - 偏程序输入
  - 更适合后续代码读取
- `aaak_entities.md`
  - 偏人和 agent 直接阅读
  - 更接近 AAAK / wake-up 的解释层

也就是说：

- 一个更像结构化 bootstrap 数据
- 一个更像可读的 identity / registry 文档

所以这轮不是“多存一份同样的东西”，而是在补两个不同接口面。

## 2. `critical_facts.md` 的价值在于“先给出一个可改的起点”

真正的长期 facts 当然应该来自：

- 挖掘后的 drawer
- 更完整的 registry / KG
- 后续人工补充

但第一次接入时最难的是“完全没有起点”。

所以这轮的 `critical_facts.md` 只做 bootstrap：

- People
- Projects
- Palace

它不是终点，而是一个：

- 可读
- 可改
- 以后能继续 enrich

的起点层。

## 3. local-first 路线下，这类 bootstrap 文档更适合放在项目目录，而不是全局 home

Python onboarding 默认更偏 `~/.mempalace/...`。

Rust 这里延续当前重写线的 local-first 思路，把 bootstrap 文档放在项目目录：

- `project/aaak_entities.md`
- `project/critical_facts.md`

这样做的好处是：

- 项目迁移时更完整
- 多项目不会共用一份全局 onboarding 文档
- review 和 handoff 都更容易局部化

# 补充知识

## 1. “已有文件不覆盖” 对文档型 bootstrap 比对配置型 bootstrap 还重要

配置文件被覆盖，用户通常还能重跑或修回来。

但文档型 bootstrap 一旦被覆盖，用户后面手改过的说明、缩写、注释很容易直接丢掉。

所以这轮沿用上一轮的策略：

- 文件不存在：生成
- 文件已存在：保留

对 `aaak_entities.md` / `critical_facts.md` 来说，这比“每次 init 都保持最新”更重要。

## 2. `entity_code()` 这种小工具看着简单，但决定了 bootstrap 文档是否稳定

AAAK registry 里需要简短 code，比如：

- `JOR=Jordan`
- `ATLA=Atlas`

如果没有稳定 code 规则，文档就会：

- 每次生成长得不一样
- 不利于 review
- 不利于后面 agent 按 code 引用

所以这轮单独提了 `entity_code()`，让这个规则在 Rust 里变成显式实现，而不是散在字符串拼接里。

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

后续提交前还会再跑一轮完整：

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这轮没有迁 Python interactive onboarding 问答流，只补了非交互 bootstrap 产物。
- 这轮没有实现更完整的 entity registry 生命周期，只是先把文档 bootstrap 写出来。
- `critical_facts.md` 目前仍是“初始化起点”，还没有接后续自动 enrich。
- 这轮没有改 MCP，也没有给写面加新的 onboarding 工具。
