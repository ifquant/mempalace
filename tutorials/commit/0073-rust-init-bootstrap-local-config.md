# 背景

Rust 版 `mempalace` 之前已经有 `init`，但它做的事情比较薄：

- 建 palace 目录
- 初始化 SQLite / LanceDB
- 返回一个基础 summary

这和 Python 版的 `init` 还有一个明显差距：Python 的 `init` 不只是“建库”，它还会在项目目录里做本地 bootstrap：

- 扫描项目结构，生成 `mempalace.yaml`
- 扫描文本内容，尽量生成 `entities.json`

这两步很重要，因为它们会直接影响后续：

- `mine` 的 room 路由
- `compress` / AAAK 的实体配置
- 整个项目第一次接入时的可用性

所以这一提交的目标，不是继续扩新命令，而是把 Rust `init` 补成真正接近 Python 的“本地初始化 + 项目 bootstrap”闭环。

# 主要目标

- 给 Rust `init` 增加项目 bootstrap。
- 自动生成项目级 `mempalace.yaml`。
- 在有足够信号时生成 `entities.json`。
- 保留已有配置文件，不做覆盖。
- 把 bootstrap 结果带进 `init` 的 JSON / human summary。
- 补齐 service / CLI 回归和 README。

# 改动概览

- 新增 [rust/src/bootstrap.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/bootstrap.rs)
  - 实现项目 bootstrap 主逻辑：
    - room 检测
    - entity 检测
    - `mempalace.yaml` 写入
    - `entities.json` 写入
    - 已有文件保护
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `bootstrap` 模块
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - 新增 `App::init_project(&Path)`
  - `init_project()` 会：
    - 初始化 palace
    - 运行 bootstrap
    - 返回更完整的 `InitSummary`
  - 原有 `App::init()` 保留，用于纯 palace 初始化场景
- 更新 [rust/src/model.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/model.rs)
  - `InitSummary` 新增：
    - `project_path`
    - `wing`
    - `configured_rooms`
    - `detected_people`
    - `detected_projects`
    - `config_path`
    - `config_written`
    - `entities_path`
    - `entities_written`
- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - `init <dir>` 现在走 `init_project()`
  - `init --human` 会打印项目、wing、rooms、config/entities 路径
  - 补了 `--yes` 兼容参数，用于对齐 Python 表面；Rust 当前仍然是非交互 bootstrap
- 更新测试：
  - [rust/tests/service_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/service_integration.rs)
  - [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
- 更新 [rust/src/split.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/split.rs)
  - 顺手修了一个 `clippy` 提示的连续 `replace()` lint，保持工作树继续满足 `-D warnings`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 1. `init` 真正有价值的部分不是“建库”，而是“第一次把项目解释成 palace”

很多系统的 `init` 只是：

- 创建目录
- 创建数据库

但对 MemPalace 来说，更关键的是把一个普通项目目录转换成“可以被 memory 系统理解的目录”。

也就是：

- 这个项目叫什么 wing
- 里面可能有哪些 rooms
- 哪些名字更像人，哪些更像项目

所以 Rust 这里把 bootstrap 作为 `init` 的正式一部分，而不是留到以后手工补文件。

## 2. bootstrap 要优先“保守写入”，不要覆盖用户已经改过的配置

`mempalace.yaml` 和 `entities.json` 都是用户后续很可能会人工改的文件。

所以这轮实现的策略是：

- 文件不存在：自动生成
- 文件已存在：读取并保留，不覆盖

这和“每次 init 都重写”完全不是一个风险等级。对本地工具来说，保留用户已有配置通常比“永远自动最新”更重要。

## 3. entity 检测先做启发式，不要为了对齐 Python 把交互逻辑硬抄进 Rust

Python 的 `entity_detector.py` 有一整套：

- 候选名提取
- score / classify
- interactive confirm

Rust 这轮只迁了前半段的核心价值：

- 从 prose/readable 文件里扫候选
- 用人称动词、对话标记、项目提示词做轻量分类
- 生成可直接落盘的 `entities.json`

没有把交互确认一起搬过来，是有意的：当前 Rust CLI 其它命令大多已经走稳定 JSON / human 输出风格，先把非交互 bootstrap 做稳，更符合这条重写线。

# 补充知识

## 1. room 检测和 room 路由是两回事

`init` 里生成的 rooms 只是“taxonomy bootstrap”，它不直接决定所有文件最终一定落哪个 room。

后面的 `mine` 仍然会结合：

- 路径
- 文件名
- 内容关键词

来实际路由 drawer。

也就是说：

- `init` 负责先把房间建出来
- `mine` 负责决定文件真正进哪个房间

这两个阶段要分开看。

## 2. 测试里不要把 `entities.json` 当成“永远会生成”

这轮一个真实回归就是：

- 某个 fixture 只有 auth 文本
- room bootstrap 能稳定生成
- 但 entity 检测不一定有足够信号

如果测试强断言 `entities_written == true`，就会把合理的“未检测到实体”误判成 bug。

所以更稳的测试方法是分两类：

- 普通 round-trip：只断言稳定存在的 `mempalace.yaml`
- 专门的 entity fixture：给足 `Jordan / Atlas` 这类信号，再断言 `entities.json` 会写出

这类测试分层在启发式逻辑里很重要。

# 验证

本次实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

本次新增/覆盖的关键回归包括：

- `bootstrap::tests::bootstrap_detects_rooms_and_entities_and_writes_files`
- `bootstrap::tests::bootstrap_preserves_existing_files`
- `init_project_bootstraps_rooms_and_entities`
- `cli_init_writes_entities_json_when_detection_finds_names`
- `cli_init_status_mine_search_round_trip`
- `cli_init_human_prints_python_style_summary`

# 未覆盖项

- 这轮没有迁 Python `entity_detector.py` 的交互确认流程，Rust 仍然是非交互 bootstrap。
- 这轮没有迁完整 onboarding / `aaak_entities.md` / `critical_facts.md` 生成链路。
- Rust 目前只把 `entities.json` 作为 bootstrap 产物写出来，还没有把它继续接进更深的 entity registry 生命周期。
- `split` 只是顺手修了一个 lint，没有做新的功能扩展。
