# 0153 Rust 对齐总账第一轮落地

## 背景

前面 Rust 重写已经连续推进了很多轮，主能力面其实已经很大程度上补齐了。但仓库里的叙述还保留着一些更早阶段的判断，比如：

- 某些段落还在暗示 Rust 还缺一块 Python MCP 写面
- README 里“第一阶段还没做什么”的说法，已经和当前代码现实不完全一致

如果继续按这些旧判断推进，就会把后续对齐工作带偏。

## 主要目标

把当前 Rust/Python 的用户可见对齐状态先写成一份实盘总账，同时把 `rust/README.md` 里已经过时的剩余差距表述修正掉。

这次**不**补代码实现，只先把“现状到底是什么”固定下来。

## 改动概览

- 新增 [docs/parity-ledger.md](/Users/dev/workspace2/agents_research/mempalace/docs/parity-ledger.md)
  - 记录 Python CLI surface
  - 记录 Rust CLI surface
  - 记录 Python MCP surface
  - 记录 Rust MCP surface
  - 用固定结论词标记：`aligned` / `rust superset` / `intentional divergence` / `remaining`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 增加当前 parity 状态摘要
  - 增加指向 parity ledger 的链接
  - 删除已失真的 “remaining Python write MCP surface ...” 表述
  - 保留“当前 Rust 不直接兼容 Python palace 数据”的边界

## 关键知识

### 1. 先做 parity ledger，再做 parity implementation

当一个重写项目进入后期，很容易继续沿着旧印象补东西，比如“我记得这块还没做”。  
更稳的做法是先写一张总账，把每个表面到底是：

- 已对齐
- Rust 超集
- 有意偏离
- 还剩残项

明确下来。这样后续每一轮实现，都会更像是“按账销项”，而不是“凭感觉补洞”。

### 2. Rust 超集不是 parity 缺口

这次盘点的一个重要结论是：

- Python CLI 公开面已经小于 Rust CLI
- Python MCP 工具面也已经小于 Rust MCP

所以像 `onboarding`、`normalize`、`registry_*`、`layers_status`、`repair_*` 这些 Rust 新表面，应该记成 `rust superset`，而不是“还没对齐完”。

### 3. 有意偏离要写成边界，不要写成 TODO

“Rust 不直接兼容 Python 旧 palace 数据”这件事，如果写成模糊的 future work，后面很容易被误解成漏做了。  
更清楚的写法是：这属于当前阶段的 `intentional divergence`，也就是明确边界，而不是隐含 bug。

## 补充知识

### 1. 文档债会反向污染实现节奏

很多时候不是代码缺口在拖慢项目，而是旧文档还在描述更早的状态。  
如果不先修文档，后面每次“继续”都可能继续沿着过时假设推进。

### 2. “功能存在” 和 “行为完全一致” 是两层不同的 parity

这一轮只先确认了：

- 命令/工具是否存在
- 表面范围谁覆盖了谁

这不等于所有边界行为都已经完全一致。  
后面的 residual parity 批次，才适合继续审更细的行为差异。

## 验证

这次是文档真相收口，不涉及 Rust 代码实现改动。验证方式是源码交叉检查：

- 对照 `python/mempalace/cli.py` 和 `rust/src/root_cli.rs`
- 对照 `python/mempalace/mcp_server.py` 和 Rust `mcp_schema_*` / `mcp_runtime_*`
- 确认 `docs/parity-ledger.md` 与 `rust/README.md` 的结论一致

## 未覆盖项

- 这次没有修改 `rust/src/` 下任何实现代码
- 这次没有开始 residual parity 的第二波实现切片
- 这次没有处理 Python 旧 palace 数据兼容
