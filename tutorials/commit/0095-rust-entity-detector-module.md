## 背景

Rust 版前面已经有实体检测能力，但它一直藏在 `bootstrap.rs` 里面：

- `init` 会用它
- `onboarding` 会用它
- `registry learn` 也会间接用它

问题是，这样它更像“bootstrap 的内部细节”，而不是一个真正对齐 Python
`entity_detector.py` 的公共模块。

Python 版这块是明确独立的：

- 扫描候选文件
- 检测 people / projects
- 作为 init 前置世界建模的一部分

如果 Rust 想继续往“库层对齐”推进，这块也应该从 `bootstrap.rs` 里拆出来。

## 主要目标

- 给 Rust 新增独立 `entity_detector` 模块
- 把文件扫描和 people/project 检测逻辑搬进去
- 保留现有行为，不重写 bootstrap 世界
- 让 `bootstrap/onboarding/service` 都改用这层公共 API
- 把这块能力写进 README

## 改动概览

- 新增 [rust/src/entity_detector.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/entity_detector.rs)
  - `DetectedEntities`
  - `detect_entities()`
  - `detect_entities_for_registry()`
  - `scan_for_detection()`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `entity_detector`
- 更新 [rust/src/bootstrap.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/bootstrap.rs)
  - 去掉内嵌检测实现，改用新模块
- 更新 [rust/src/onboarding.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/onboarding.rs)
  - 从 `entity_detector` 调 `detect_entities_for_registry()`
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - `registry_learn()` 改走新模块
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. 从 bootstrap 拆出 detector，不是“重构癖”，而是边界修正

这次不是为了拆文件而拆文件。问题本质是：

- `bootstrap` 的职责应该是“写配置 / 写 bootstrap 文件”
- `entity detector` 的职责应该是“从项目文本里发现人和项目”

如果两者混在一起，后面任何想复用 detector 的地方都会反过来依赖 bootstrap。
这会让模块边界越来越奇怪。

### 2. detector API 应该保留两层

这次刻意保留了两个常用入口：

- `detect_entities()`
  - 返回完整检测结果
  - 包含 `files_scanned`
- `detect_entities_for_registry()`
  - 返回 `(people, projects)`
  - 给 bootstrap / onboarding / registry 这种调用方用起来更轻

如果只保留一个“最底层”接口，所有调用方都得自己拆结果，代码会重新散开。

## 补充知识

### 1. 拆公共模块时，先搬逻辑，再改调用点，风险最低

这次做法是：

1. 先把 detection 逻辑完整搬到新模块
2. 再让 `bootstrap/onboarding/service` 改用它
3. 最后删掉旧实现

这种顺序的好处是：

- 很容易做 diff 对照
- 行为回归更容易看出是不是“搬坏了”
- 调用点变更和算法变更不会混成一坨

### 2. README 里把“模块级对齐”写出来很重要

最近这几轮做的不是单纯功能，而是 Rust 库层 shape 对齐：

- `palace`
- `layers`
- `searcher`
- 这次的 `entity_detector`

如果不把这些写进 README，后来的 agent 很容易只看到 CLI/MCP，忽略 Rust 库层已经开始成体系了。

## 验证

实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

本次新增覆盖了：

- `entity_detector` 能扫描 prose 文件并检测 people/projects
- `entity_detector` 会跳过 `target/` 之类噪声目录
- 现有 `init/onboarding/registry learn` 继续通过原有回归

## 未覆盖项

- 这次没有把 Python `confirm_entities()` 那类交互确认流迁到 Rust detector 模块
- 这次没有新增独立 CLI `entity-detect` 命令，只先把库层模块补出来
- 这次没有改 Python `entity_detector.py`，只是把 Rust 的公共 API 形状拉近它
