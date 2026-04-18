## 背景

Rust 这边已经把 conversation normalize 和 spellcheck 接进了实际 mining 主链，但外部还没有一个像 Python `normalize.py` 那样的直接入口。

这会带来一个调试问题：

- 用户拿到一份 chat export
- 想先看 normalize 之后会变成什么
- 现在只能真的跑 `mine --mode convos`

这对排查 transcript schema、spellcheck、room 路由都不够直接。

## 主要目标

- 给 Rust 增加 `normalize <file>` CLI
- 默认输出结构化 JSON
- `--human` 输出人类可读预览
- 复用已经存在的 normalize + spellcheck 逻辑，不复制一套旁路实现

## 改动概览

- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - 新增 `normalize` 子命令
  - 默认输出：
    - `kind`
    - `file_path`
    - `changed`
    - `chars`
    - `quote_turns`
    - `normalized`
  - `--human` 输出：
    - 文件路径
    - 是否改动
    - 字符数
    - user turn 数
    - 前 12 行 preview
- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - `cli_normalize_help_mentions_chat_export_normalization`
  - `cli_normalize_json_reports_changed_transcript`
  - `cli_normalize_human_prints_preview`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. normalize CLI 不应该再实现一套新逻辑

这轮最重要的约束是：`normalize` 只是主链的一个观察窗，不是第二套 normalize 系统。

所以 CLI 直接复用：

- [rust/src/convo.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/convo.rs) 里的 `normalize_conversation_file()`

这样才能保证：

- `normalize` 看见的结果
- `mine --mode convos` 真正写入 palace 的结果

是同一份事实。

### 2. `changed` 字段很重要

只输出 normalized 文本还不够，因为很多文件本来就是：

- 已经是 `>` transcript
- 或者根本不需要 schema 变换

所以这里专门输出 `changed`，用户能立刻知道：

- 这份输入是否只是 pass-through
- 还是确实被转换/修正了

### 3. `--human` 不是替代 JSON，而是补充调试入口

默认保持 JSON，原因和前面一致：

- 稳定
- 可测试
- 可脚本化

`--human` 只是让本地排查更快：

- 看 preview
- 看 quote turn 数
- 看 spellcheck 是否生效

## 补充知识

### 为什么 preview 只截前 12 行

normalize 后的 transcript 可能很长。  
human 模式的目标是“快速判断结果是不是对的”，不是把整个 transcript 再打印一遍。

所以这里直接做短 preview，更适合终端调试。

### 为什么 unsupported 文件返回稳定错误

这条命令主要就是拿来排查导出格式的。  
如果遇到：

- 不可读文件
- 不支持的 JSON 结构
- 空文件

最糟糕的体验就是 panic 或 silent pass-through。

这轮做法是：

- JSON 模式返回结构化 error
- human 模式返回可读文本错误

方便后面继续补 schema 支持时做回归。

## 验证

- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test`
- `cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 还没有把 `normalize` 暴露成 MCP tool
- 还没有加 `--output-file`
- 还没有给 normalize CLI 做 bulk directory 模式
- unsupported schema 目前还是统一归到错误，没有更细分的 parser diagnostics
