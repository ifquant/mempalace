# 0001 Rust bootstrap 与协作规则初始化

## 背景

这个仓库原本只有 `python/` 主实现，最近开始把未来重写方向放到 `rust/`。一旦进入“双实现并存”阶段，如果没有明确协作规则，AI 很容易犯两类错误：

- 把 `python/` 当成整个仓库的唯一实现，忽略 `rust/`
- 反过来把 Rust 规则误套到 Python 维护工作里

所以这次提交的重点不是加业务功能，而是把协作边界、提交规则、教程规则先固定下来。

## 主要目标

- 让 `AGENTS.md` 真实反映当前仓库结构
- 为后续 Rust 重写建立可执行的提交与教程流程
- 明确 Rust 是性能敏感实现，需要保留低开销接口层

## 改动概览

- 重写仓库根部的 `AGENTS.md`
- 新增 `tutorials/commit/0001-rust-bootstrap-rules.md` 作为后续提交教程示例
- 在规则中区分 `python/` 与 `rust/` 的职责和验证命令
- 加入自动提交边界与高信号 commit message 规范

## 关键知识

这个仓库目前有两条不同成熟度的实现线：

- `python/`：现有可运行实现，CI 已覆盖
- `rust/`：重写起点，当前只建立了依赖和最小 crate

这意味着协作规则必须按“目标子树”来写，而不是写成统一的大而泛规则。否则后续 agent 很容易：

- 在 Rust 任务里引用 Python 测试命令作为“已验证”
- 在 Python 稳定面上做了不该做的破坏性改动

另外，Rust 这里不是普通脚本仓库，而是偏性能敏感的数据/检索方向。规则中要求保留低开销接口层，是为了防止未来只留下易用但昂贵的接口，例如：

- 每次调用都强制分配新 `Vec`
- 批处理路径只能走高层 stream 抽象
- 热路径必须经过多层 boxing / conversion

## 补充知识

1. 设计规则时，优先写“下一位 agent 最容易误判的边界”，比写一堆通用好习惯更有价值。  
   这里最容易误判的边界就是：`python/` 维护和 `rust/` 重写不是一回事。

2. 对 Rust 性能工作来说，优化不只是换更快算法。  
   API 形状本身也会决定性能上限，比如是否允许 caller-provided buffers、是否支持 batch 接口、是否减少中间分配。

## 验证

- 读取并对照了当前仓库结构
- 检查了 `python/pyproject.toml`、`rust/Cargo.toml`、`.github/workflows/ci.yml`
- 之前已实际运行过：
  - `cd rust && cargo check`

## 未覆盖项

- 这次没有新增 Rust CI workflow
- 这次没有创建 `rust/` 子目录自己的独立 `AGENTS.md`
- 这次没有处理 Python 与 Rust 的功能对齐计划
