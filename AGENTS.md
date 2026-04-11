# AGENTS.md

> MemPalace 仓库协作规则。先遵守上层 `/Users/dev/workspace2/agents_research/AGENTS.md`，本文件只补充本仓库的局部规则。

## 项目概览

当前仓库处于双轨阶段：

- `python/` 是现有可运行实现，也是当前 CI 覆盖的主实现
- `rust/` 是新的本地优先重写方向，当前采用 `LanceDB + rusqlite` 路线

默认目标不是把整个仓库当成单一代码库处理，而是先判断任务落在：

- `python/` 现有实现维护
- `rust/` 重写与架构搭建
- 仓库级配置与文档

## 适用范围

本文件作用域为 `/Users/dev/workspace2/agents_research/mempalace`。

如果后续在 `python/` 或 `rust/` 下新增子级 `AGENTS.md`，子级规则优先生效。

## 目录导航

- `python/mempalace/`: 现有 Python 包实现
- `python/tests/`: Python 测试
- `python/benchmarks/`: Python benchmark 脚本
- `python/examples/`: Python 示例与集成文档
- `rust/`: Rust 重写入口
- `rust/src/`: Rust crate 代码
- `hooks/`: Claude / agent hook 脚本
- `docs/`: SQL 与补充资料
- `.github/workflows/`: CI 配置
- `tutorials/commit/`: 每次提交对应的教程文档

忽略高噪声目录，除非任务明确要求：

- `rust/target/`
- `python/.pytest_cache/`
- `python/.ruff_cache/`

## 常用命令

### Python

```bash
cd python
pip install -e ".[dev]"
python -m pytest tests/ -v --ignore=tests/benchmarks
python -m pytest tests/ -v --ignore=tests/benchmarks --cov=mempalace --cov-report=term-missing
ruff check .
ruff format .
ruff format --check .
```

### Rust

```bash
cd rust
cargo check
cargo test
cargo fmt
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

### 仓库级快速检查

```bash
git status --short
find tutorials/commit -maxdepth 1 -type f | sort
```

## 开发原则

- 先确认目标子树，再开始做 substantial work。不要默认同时改 `python/` 和 `rust/`。
- Python 任务优先保持现有行为兼容，除非任务明确允许改变 CLI、存储格式或 MCP 行为。
- Rust 任务优先为重写搭基础设施、边界层和验证路径，不要假装已经功能对齐 Python。
- 仓库现在仍以本地优先为核心设计：不要引入必须依赖远程服务的主路径。
- 文档、CI、脚本如果只覆盖 `python/`，不要在未验证前声称已经支持 `rust/`。

## 代码约定

### Python

- 使用 `ruff` 风格，双引号，snake_case / PascalCase。
- 测试文件放在 `python/tests/test_*.py`。
- Python 包入口与行为兼容面主要在：
  - `python/mempalace/cli.py`
  - `python/mempalace/mcp_server.py`
  - `python/mempalace/config.py`

### Rust

- 当前 crate 在 `rust/Cargo.toml`，包名 `mempalace-rs`。
- 依赖主线是 `lancedb`、`rusqlite`、`clap`、`tracing`。
- Rust API 默认要求分层：
  - 可以有易用的高层接口
  - 热路径必须保留低开销接口，不能只剩分配密集、抽象过厚的 convenience API
- 批量处理不要默认强行包装成 stream-style 状态机；如果会引入明显额外开销，应优先提供直接批处理路径。
- 任何性能工作都要把 API 形状、分配次数、拷贝次数视为一等问题，而不是只看算法内核。

## 应用领域硬约束

- Python 现有架构仍是 `vector store + SQLite knowledge graph`。若改 schema、目录布局或检索语义，必须先确认。
- Rust 重写当前默认方向是：
  - `LanceDB` 负责本地向量检索
  - `rusqlite` 负责关系型状态和 knowledge graph
- 不要在没有迁移计划的情况下让 Rust 直接复写或破坏 Python 既有本地数据目录。
- `hooks/` 涉及自动保存与用户本地环境，修改前必须确认风险。

## 测试与验收

- 改 Python 代码时，至少运行与改动直接相关的 `pytest` 文件；能跑全量单测时优先跑全量非 benchmark 集。
- 改 Rust 代码时，至少运行 `cargo check`；如果涉及可执行逻辑或公共 API，优先补 `cargo test`。
- 改 `.github/workflows/`、打包、版本或安装文档时，至少做一轮对应本地命令验证。
- 在文档里写到某条命令时，优先运行最低成本的真实命令，而不是只凭推测。

## 禁止事项

- 不要把 `rust/target/`、构建产物或缓存文件加入版本控制。
- 不要无理由同时重写 Python 和 Rust 两条实现。
- 不要用模糊 commit message，例如 `fix`、`update`、`cleanup`、`misc`，除非改动确实极小。
- 不要在未验证兼容性的情况下修改用户可见接口：
  - CLI flags
  - MCP tool names / params
  - 本地数据目录结构
  - hook 行为

## 需要先确认的情况

遇到下列情况先问用户，不要直接拍板：

- 改变 Python CLI / MCP 的外部行为
- 改变 Python 或 Rust 的持久化格式 / schema / on-disk layout
- 改动 `.github/workflows/` 使 CI 范围新增 Rust 编译或测试
- 改动 `hooks/`、安装步骤或面向用户的默认路径
- 删除已有实现、迁移目录、或让 Rust 取代 Python 成为默认入口

## 提交 / PR 要求

### Commit message

保持 conventional commits 风格，但信息密度必须足够高。

非平凡提交至少要说明：

- 为什么要改
- 改了什么
- 如何验证
- 明确没做什么

推荐格式：

```text
feat: add rust lancedb bootstrap

Changes:
- add initial rust crate under rust/
- switch local vector backend choice to LanceDB
- document Rust bootstrap constraints in AGENTS.md

Verification:
- cd rust && cargo check

Not included:
- no Rust CI workflow yet
- no Python/Rust parity work yet
```

### 自动提交规则

- 每次做完一个清晰功能切片后，如果代码、验证、教程文档都已完成，AI 应直接创建 commit，不必等待用户再提醒一次。
- 一个“清晰功能切片”通常指：
  - 单一目标
  - 范围边界明确
  - 已完成最低成本真实验证
  - 已写好本次提交对应的 `tutorials/commit/NNNN-*.md`
- 不要把多个无关目标塞进同一个自动提交里。

### Commit 教程规则

- 每次非平凡 commit，都要新增一个 `tutorials/commit/NNNN-*.md`。
- 编号使用四位递增，从 `0001` 开始，不能跳号或复用。
- 教程默认用中文写给新同学看，但命令、路径、API 名、代码符号保持原文。
- 教程必须覆盖以下章节：
  - `背景`
  - `主要目标`
  - `改动概览`
  - `关键知识`
  - `补充知识`
  - `验证`
  - `未覆盖项`
- `补充知识` 默认加入 1 到 2 条来自本次真实实现过程的新人友好知识点，例如：
  - Rust / Python 语言点
  - API 分层与性能设计
  - 调试习惯
  - 与 agent 协作时有用的 prompt 技巧

## 参考资料

- 仓库概览：`README.md`
- Python 说明：`python/README.md`
- Python 提交说明：`python/CONTRIBUTING.md`
- Rust 依赖起点：`rust/Cargo.toml`
- Rust 重写说明：`rust/README.md`

## 子目录约定

- `python/`：保持现有实现可维护、可测试、可发布
- `rust/`：优先搭基础设施、存储层、CLI 骨架与验证链路
- `tutorials/commit/`：为每次非平凡提交提供配套教程

## 完成任务前

- 明确说明你工作在哪个子树。
- 列出改动文件。
- 说明你刻意没有修改的相邻区域。
