# 背景

Rust 版之前已经有 `mcp --setup`，能打印 Python 风格的 MCP 安装指令。

但默认行为还是 Rust 自己的一套：

- `mcp` 直接启动 stdio server
- `mcp --setup` 才打印 setup

这和 Python CLI 不一致。Python 的 `mempalace mcp` 默认就是打印 quick setup。

这类差异虽然不大，但它是**用户第一眼就会遇到的外部行为**，所以值得单独收口。

# 主要目标

1. 把 Rust `mcp` 的默认行为改成和 Python 一致
2. 保留显式的 server 启动入口，避免功能倒退
3. 让 help、setup 文案、默认行为三者完全一致

# 改动概览

- 更新 [rust/src/main.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/main.rs)
  - `mcp` 子命令新增 `--serve`
  - 默认 `mcp` 现在打印 quick setup
  - `mcp --serve` 才显式启动 stdio server
  - setup 文案里的示例命令也改成了 `mempalace-rs mcp --serve ...`
- 更新 [rust/tests/cli_integration.rs](/Users/dev/workspace2/agents_research/mempalace/rust/tests/cli_integration.rs)
  - 覆盖新的 help 文案
  - 覆盖默认 `mcp` 输出
  - 覆盖 `mcp --setup` 兼容输出
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

# 关键知识

## 1. “默认行为”比“有没有这个 flag”更重要

命令行兼容性不只是看：

- 有没有 `--setup`
- 有没有 `--serve`

还要看用户最自然输入的命令是什么。

Python 用户会直接敲：

```bash
mempalace mcp
```

如果 Rust 默认不是这个行为，即使功能都在，迁移体验也还是不一致。

## 2. 保留显式 `--serve` 是为了把“说明”和“执行”分开

把默认 `mcp` 改成 setup 之后，如果还想保留 server 启动能力，最稳的办法就是显式加一个动作开关：

```bash
mempalace-rs mcp --serve
```

这样：

- 默认命令更安全
- 用户不会误起一个阻塞式 stdio server
- 文案也更容易写清楚

# 补充知识

1. CLI 兼容改动里，**帮助文本本身也要测试**。  
   否则你可能已经改了行为，但 `--help` 还在教旧用法，最后用户仍然会踩坑。

2. 有些命令不适合直接在测试里“真跑成功路径”，例如 stdio server。  
   这时更好的做法是：
   - 测默认行为
   - 测 help
   - 测 setup 文案  
   而不是让测试卡在一个长生命周期进程上。

# 验证

执行过：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

并特别验证了：

```bash
cargo test --test cli_integration cli_mcp_
```

# 未覆盖项

- 没有新增 `mcp --serve` 的阻塞式集成测试
- 没有改 MCP server 本体协议和工具实现
- 没有动 Python 侧命令行为，只是把 Rust 默认行为向 Python 靠齐
