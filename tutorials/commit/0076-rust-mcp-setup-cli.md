# 背景

Rust 版 `mempalace` 前面已经把 MCP server 本体补起来了：

- `mcp` 命令可以直接跑 stdio server
- 读写 tool surface 也已经迁了很多

但和 Python CLI 对照，还有一个非常具体的小缺口：

- Python 有 `mcp` setup/help 输出
- Rust 只有“直接跑 server”

这会带来一个实际问题：  
即使功能已经存在，第一次接入的人还是得自己猜：

- 怎么把 Rust server 加到 Claude MCP
- 有 palace 路径时命令应该怎么写
- 直接运行 server 的命令长什么样

所以这一提交的目标不是扩 MCP 能力本身，而是把 Python 那个很实用的 `mcp` setup 表面补到 Rust CLI。

# 主要目标

- 给 Rust `mcp` 增加 `--setup`
- 保持现有 `mcp` 默认行为不变，仍然直接跑 stdio server
- `--setup` 输出 Python 风格 quick setup 指引
- 支持带 `--palace` 的路径提示
- 补齐 CLI 回归和 README

# 改动概览

- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - `Command::Mcp` 从无参数扩成：
    - `mcp`
    - `mcp --setup`
  - 默认 `mcp` 仍然执行：
    - `mcp::run_stdio(config)`
  - 新增：
    - `print_mcp_setup()`
    - `shell_quote()`
  - `mcp --setup` 现在会打印：
    - `MemPalace MCP quick setup:`
    - `claude mcp add mempalace -- ...`
    - `Run the server directly:`
    - `Optional custom palace:`
- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - 新增：
    - `cli_mcp_help_mentions_setup_flag`
    - `cli_mcp_setup_prints_python_style_quick_setup`
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)
  - 把 `mcp --setup` 写成当前 Rust CLI 的能力事实

# 关键知识

## 1. 这轮的重点不是“多一个 flag”，而是把 server 接入路径显式化

很多 CLI 功能都已经做完了，但如果第一次使用的人还得自己拼启动命令，那么真实可用性还是差一截。

`mcp --setup` 的价值在于：

- 不需要再翻 README 找命令
- 不需要手动猜 `claude mcp add ...` 的格式
- 不需要自己拼 `--palace` 路径

所以这里的价值在“降低接入摩擦”，不是算法或存储层。

## 2. 保留 `mcp` 默认直接跑 server，比改成默认打印帮助更稳

当前 Rust 已经把：

- `mempalace-rs mcp`

当成真正的 server 入口。

如果这轮直接把默认行为改成“只打印 setup”，会把已有脚本和手工调用都打断。

所以这里选的是更稳的兼容方案：

- `mcp`：继续跑 server
- `mcp --setup`：打印 Python 风格 setup 指引

这类“新功能尽量不打断旧行为”的决策，在 CLI 表面收口时特别重要。

## 3. shell quote 不是可选细节

`--palace` 路径里很容易出现空格，比如：

- `/tmp/palace path`

如果 setup 输出不做 quoting，用户复制过去就会直接失败。

所以这轮单独补了 `shell_quote()`，把 setup 输出做成可直接复制运行的命令，而不是“长得像命令”的字符串。

# 补充知识

## 1. CLI surface 对齐通常比核心功能对齐更容易被忽略

很多重写项目会优先做：

- 数据结构
- 存储
- 算法

但最后真正影响“第一次能不能用起来”的，往往是：

- help 文案
- setup 提示
- 错误路径
- 默认输出 shape

`mcp --setup` 就是典型例子：功能本身不重，但它明显提高了真实接入体验。

## 2. 这类 setup 输出最好保持“最小但完整”

setup 文案如果写得太长，反而没人愿意看；如果太短，又缺关键步骤。

Python 那个版本的好处是结构非常实用：

1. quick setup
2. direct run
3. optional custom palace

Rust 这里直接复用这个结构，而不是重新发明一套话术，这样也更利于对齐用户心智。

# 验证

本次实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo test --test cli_integration cli_mcp_help_mentions_setup_flag
cargo test --test cli_integration cli_mcp_setup_prints_python_style_quick_setup
```

提交前还会再跑完整验证：

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这轮没有改 MCP server 本体协议，只补了 CLI setup surface。
- 这轮没有做 host-specific setup（比如不同 MCP client 的多套文案），只先对齐 Python 当前的 Claude 风格提示。
- `mcp --setup` 目前只输出文本，不做自动写配置文件。
