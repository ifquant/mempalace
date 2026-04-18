# 背景

Rust 版 `mempalace-rs` 之前已经有不少 onboarding 相关能力，但分散在 `init` 和 `registry` 子命令里：

- `init` 会尽量写出 `mempalace.yaml`
- `init` 也会补 `entity_registry.json`
- `registry` 则负责后续增量维护

这已经够用，但还不像 Python 里的 `onboarding.py` 那样，给用户一个单独的“先把我的世界告诉 MemPalace”入口。

这次提交的目标，就是把这条“首次世界建模”链路正式补到 Rust CLI。

# 主要目标

1. 新增独立 `onboarding <dir>` 命令
2. 支持非交互参数模式，方便脚本和测试调用
3. 支持交互式提问，贴近 Python onboarding 的使用体验
4. 把 onboarding 真正落盘到项目本地：
   - `mempalace.yaml`
   - `entities.json`
   - `entity_registry.json`
   - `aaak_entities.md`
   - `critical_facts.md`
5. 保持 Rust 当前“项目本地优先”的路径约定，不回退到 Python 的 `~/.mempalace`

# 改动概览

- 新增 [rust/src/onboarding.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/onboarding.rs)
  - 定义 `OnboardingRequest`
  - 实现交互式提问
  - 实现 `--person` / `--project` / `--alias` 参数解析
  - 支持 `--scan` 自动补充本地检测到的人名/项目名
- 扩展 [rust/src/model.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/model.rs)
  - 新增 `OnboardingSummary`
- 扩展 [rust/src/bootstrap.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/bootstrap.rs)
  - 把已有 bootstrap 写文件 helper 提升成可复用函数
  - 新增 `write_project_config_from_names`
- 扩展 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - 新增 `onboarding` CLI 子命令
  - 新增 JSON / `--human` 输出
  - 新增 onboarding 专属错误输出
- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - 覆盖 help、JSON bootstrap、human summary
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 1. 为什么要单独做 `onboarding`，而不是继续往 `init` 里塞

`init` 更像“建 palace + 建项目配置”。

`onboarding` 更像“把用户的世界观先告诉系统”：

- 谁是重要的人
- 哪些名字是项目
- 有哪些别名
- 应该怎样分 wings

把这两条职责拆开，CLI 会更清楚，也更接近 Python 仓库里 `onboarding.py` 的语义。

## 2. 为什么 onboarding 不应该依赖 embedding/runtime

这次实现故意没有把 `onboarding` 建在 `App::new()` 那条需要 embed provider 的路径上。

原因很简单：用户第一次做 onboarding 时，根本不需要：

- 加载 `fastembed`
- 检查 LanceDB
- 检查 palace SQLite schema

onboarding 只是本地 bootstrap 文档和 registry 的写入。如果它被 embedding 配置拖死，用户体验会非常差。

## 3. Rust 版为什么把 onboarding 文件写到项目本地

Python 原版偏向 `~/.mempalace`。

Rust 这条重写线现在的核心约束是 **local-first + palace-local + project-local**，所以 onboarding 也延续这个方向，直接写进目标项目目录。这能避免：

- 全局状态互相污染
- 多项目上下文串味
- 调试时不知道文件到底落在哪

# 补充知识

1. `std::io::IsTerminal` 很适合做 CLI 的“自动交互 / 自动非交互”分流。  
   如果 `stdin/stdout` 不是 terminal，就不要偷偷等待用户输入，否则测试和脚本会直接卡死。

2. 这类“既要交互又要脚本化”的命令，最好一开始就把核心逻辑收成一个请求结构体。  
   这次的 `OnboardingRequest` 就是把：
   - 交互式输入
   - CLI flag 输入
   - 测试夹具输入  
   三条入口统一到了同一层业务逻辑里。

# 验证

执行过：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

重点新增回归：

- `cli_onboarding_help_mentions_mode_people_and_scan`
- `cli_onboarding_json_bootstraps_local_world_files`
- `cli_onboarding_human_prints_setup_summary`

# 未覆盖项

- 还没有把 onboarding 做成 MCP 工具
- 还没有把 Python `onboarding.py` 里“逐个确认 auto-detect 候选”的细粒度交互完全搬齐
- 目前仍然优先采用 Rust 的项目本地文件布局，不与 Python 的 `~/.mempalace` 全局路径对齐
