# 背景

Rust 版这段时间已经把很多核心能力补起来了：

- `mine`
- `search`
- `migrate`
- `repair`
- MCP 只读工具

但从用户第一眼接触到的角度看，还有一个明显短板：

- CLI help 太薄

之前的 `clap` 默认输出虽然能列出命令，但它缺少：

- 项目介绍
- 参数说明
- 示例
- 与 Python 版接近的命令意图描述

这会直接影响：

- 新人上手
- agent 自己从 `--help` 学习命令
- 未来文档和实际 CLI 的一致性

# 主要目标

这次提交的目标是把 Rust CLI 的帮助文本往 Python 版靠一层：

1. 补根命令介绍和示例
2. 补核心子命令的 `about`
3. 补常用参数的 help 文案
4. 用测试锁住这些用户可见文本

# 改动概览

主要改动如下：

- `rust/src/main.rs`
  - 根命令 `about` 改成：
    - `MemPalace — Give your AI a memory. No API key required.`
  - 新增 `long_about`，包含当前 Rust phase 说明和使用示例
  - 为全局参数补 help：
    - `--palace`
    - `--hf-endpoint`
  - 为这些子命令补 `about`：
    - `init`
    - `mine`
    - `search`
    - `migrate`
    - `repair`
    - `status`
    - `doctor`
    - `prepare-embedding`
    - `mcp`
  - 为核心参数补 help：
    - `mine.dir`
    - `mine.wing`
    - `mine.limit`
    - `mine.no_gitignore`
    - `mine.include_ignored`
    - `search.query`
    - `search.wing`
    - `search.room`
    - `search.results`
    - `doctor.warm_embedding`
    - `prepare-embedding.attempts`
    - `prepare-embedding.wait_ms`
- `rust/tests/cli_integration.rs`
  - 新增：
    - `cli_root_help_mentions_core_commands_and_examples`
    - `cli_search_help_mentions_filters_and_results`
- `rust/README.md`
  - 记录当前 CLI help 已向 Python 入口靠近

# 关键知识

## 1. CLI help 本身就是接口的一部分

很多工程里会把帮助文本看成“后补文案”。  
但对命令行工具来说，`--help` 本身就是最直接的接口面。

尤其对 agent 来说更是这样：

- 它往往先看 help
- 再决定怎么调用命令

所以这次不是在“美化文案”，而是在补用户和 agent 真正会消费的接口层。

## 2. 帮助文本也应该有回归测试

如果 help 文案没有测试保护，很容易在后续重构里退回成：

- 只有命令名
- 没有参数说明
- 没有示例

而这种回退通常不会被普通功能测试发现。  
所以这次专门把帮助文本里最关键的几个信息点锁进了 CLI 集成测试。

# 补充知识

## 为什么这里只做“帮助文本更像 Python”，而不是强行复制全部 Python 命令

因为当前 Rust phase 还不是完全功能对齐 Python。  
如果帮助文本现在就假装所有 Python 能力都在，会误导用户。

所以这次做的是：

- 把已有能力讲清楚
- 用 Python 的表达方式和信息密度做参考

而不是伪装成“已经完全一致”。

## 为什么 `long_about` 里放示例很值

因为很多用户并不会细看每个子命令的参数帮助。  
一组简单例子通常比长解释更快建立心智模型。

比如：

- `init`
- `mine`
- `search`
- `status`
- `migrate`
- `repair`

这几个例子放在根帮助里，就足够让第一次接触的人知道主链路是什么。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增关键验证：

- `cli_root_help_mentions_core_commands_and_examples`
- `cli_search_help_mentions_filters_and_results`

这两条测试确保：

- 根 help 有项目介绍和示例
- `search --help` 有过滤和结果参数说明

# 未覆盖项

这次没有继续做：

- Rust 输出文案进一步模仿 Python 的终端样式
- `init/mine/status` 的人类友好终端排版
- 更多子命令 help 的逐项快照测试
- Python 中尚未实现到 Rust 的命令帮助

所以这次提交的定位是：  
先把 Rust CLI 的帮助面补到一个更像 Python、也更适合用户和 agent 学习的程度。
