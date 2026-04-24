# 背景

前面 Rust 版 `mempalace` 已经把：

- project / convos mining
- AAAK `compress` / `wake-up`
- `hook run`
- `instructions`

这些主链路补上了。

但 Python CLI 里还有一块很实用、而且和 convos 工作流直接相关的表面没有迁过来：

- `split`

它的作用不是“挖掘内容”，而是先把一个很长的 transcript mega-file 按 session 切成多个独立文件。这样后续：

- `mine --mode convos`
- 人工检查聊天记录
- 单次会话归档

都会更稳定。

所以这一提交的目标，是把 Rust 的 transcript splitter 做成一个真正可运行、可测试、可 dry-run 的 CLI，而不是只在 README 里写一句“以后可以补”。

# 主要目标

- 给 Rust 新增 `split <dir>` CLI。
- 对齐 Python 的 mega-file session 边界判定规则。
- 支持 dry-run 预览，不实际写文件。
- 真正写出拆分后的会话文件。
- 成功拆分后把原文件改名成 `.mega_backup`。
- 补齐 README 和 CLI 回归测试。

# 改动概览

- 新增 [rust/src/split.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/split.rs)
  - 新增：
    - `SplitFileResult`
    - `SplitSummary`
    - `split_directory()`
  - 实现：
    - 目录扫描
    - mega-file 检测
    - session boundary 查找
    - 拆分输出
    - `.mega_backup` rename
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `split` 模块
- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - 新增 CLI 子命令：
    - `split <dir>`
    - `--output-dir`
    - `--min-sessions`
    - `--dry-run`
  - 默认输出 JSON summary
- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - 新增：
    - `cli_split_help_mentions_transcript_megafiles`
    - `cli_split_dry_run_reports_output_without_writing`
    - `cli_split_writes_files_and_renames_backup`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 把 `split` 写成当前 Rust 能力事实

# 关键知识

## 1. session 边界的关键不是看到 `Claude Code v`，而是排除“恢复上下文”假阳性

如果只看到：

- `Claude Code v...`

就认定是一个新 session，很容易把下面这种内容误切开：

- `Ctrl+E to show previous messages`
- `previous messages`

这类内容常常只是历史恢复头，不是一次新的真实会话。

所以这次 Rust 里保留了 Python 的核心判断：

- 发现 `Claude Code v`
- 再往后看几行
- 如果附近出现 `Ctrl+E` 或 `previous messages`
- 就不把它当成新 session

这一步决定了 splitter 会不会把 transcript 切碎。

## 2. dry-run 不是可有可无，它是 transcript 工具的安全带

`split` 这种命令和纯查询命令不一样，因为它真的会：

- 写新文件
- 改原文件名

所以 dry-run 很重要。它让你先看：

- 哪些文件会被判成 mega-file
- 会切出几个 session
- 会生成哪些输出文件

而不马上改磁盘状态。

这类“先预览再落盘”的习惯，对 transcript / archive / migration 工具尤其重要。

## 3. 原文件改成 `.mega_backup` 比直接删除更稳

拆分成功后，如果直接删原始 mega-file，短期看起来更干净，但出问题时恢复很麻烦。

这次 Rust 采用的是：

- 成功写出 split files
- 再把原文件 rename 成 `.mega_backup`

这样有两个好处：

1. 原始内容还在，回滚简单。
2. 目录里能明显看出这个文件已经被 split 过，不会和未处理文件混在一起。

# 补充知识

## 1. transcript 文件名“可读但不脆弱”比“完美语义文件名”更重要

splitter 会从 transcript 里尝试抽：

- timestamp
- people
- subject

然后拼成新文件名。

这里不要追求“绝对完美语义命名”，因为聊天文本经常很脏。更重要的是：

- 文件名稳定
- 合法
- 大致可读

所以实现里用了 `sanitize_filename()` 做统一清洗，这是 transcript 工具里常见但很必要的一步。

## 2. 大文件扫描要有限制，不然 split 工具容易被异常输入拖垮

这次 `split` 只扫描 `.txt`，而且对单文件大小加了上限：

- `500 * 1024 * 1024`

这不是说真实 transcript 一定会有这么大，而是避免：

- 意外把超大日志当 transcript 读进来
- 或者遇到异常数据把 CLI 直接拖慢/拖死

这种 guard 对“目录扫描型工具”很值。

# 验证

本次实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

新增覆盖包括：

- `cli_split_help_mentions_transcript_megafiles`
- `cli_split_dry_run_reports_output_without_writing`
- `cli_split_writes_files_and_renames_backup`
- `split::tests::true_session_start_ignores_context_restore_headers`

# 未覆盖项

- 这轮只做了 CLI splitter，没有新增对应的 MCP surface。
- 目前只扫描 `.txt` mega-file，没有扩展到其它 transcript 容器格式。
- `people` 和 `subject` 抽取还是轻量启发式，不是完整 NLP 识别。
- 还没把 `split` 接进更高层的 onboarding / hook 自动流程；当前是显式 CLI 工具。
