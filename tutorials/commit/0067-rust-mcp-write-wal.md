# 背景

上一轮已经把 Rust 的 MCP 写工具补到了能真正改数据的程度：

- `mempalace_add_drawer`
- `mempalace_delete_drawer`
- `mempalace_kg_add`
- `mempalace_kg_invalidate`

但和 Python `mcp_server.py` 对照时，还差一块很重要的防御层：

- write-ahead log
- 审计轨迹

Python 版会在执行写操作前，把这次操作先写进 JSONL。这样就算后面出了问题，也至少知道“谁尝试写了什么”。

# 主要目标

这次提交的目标是把 Rust MCP 写工具的审计能力也补上，并保证：

- 日志逻辑不散落在每个工具里
- 所有 MCP 写工具都能统一复用
- 日志文件跟 Rust palace 放在一起，保持本地优先
- 有真实集成测试直接验证 JSONL 文件内容

# 改动概览

这次主要做了三件事。

第一件事，新增 `rust/src/audit.rs`。

这里实现了一个独立的 `WriteAheadLog`：

- `for_palace(palace_path)`
- `log(operation, params, result)`
- `file_path()`

日志文件位置是：

```text
<palace>/wal/write_log.jsonl
```

同时对目录和文件权限做了 best-effort 处理：

- 目录尽量设成 `0700`
- 文件尽量设成 `0600`

第二件事，在 `mcp.rs` 里把所有写工具接到这个审计层。

目前会写 WAL 的 MCP 工具是：

- `mempalace_add_drawer`
- `mempalace_delete_drawer`
- `mempalace_kg_add`
- `mempalace_kg_invalidate`
- `mempalace_diary_write`

而且写入时机是：

- 先记 WAL
- 再真正执行写操作

这和 Python 的“先落审计，再执行”思路一致。

第三件事，补了一条真正读取 JSONL 的 MCP 集成测试。

测试会依次调用：

1. `mempalace_add_drawer`
2. `mempalace_kg_add`
3. `mempalace_diary_write`

然后直接打开：

```text
palace/wal/write_log.jsonl
```

确认：

- 文件存在
- 一共有 3 条 JSONL
- `operation` 顺序正确
- 参数内容真的写进去了

# 关键知识

## 1. 审计日志最好做成独立模块，而不是塞在 `mcp.rs`

如果直接把：

- 打开文件
- 追加 JSONL
- 设权限

这些逻辑散在 `mcp.rs` 的每个 `match` 分支里，很快就会变成高重复代码。

单独抽成 `audit.rs` 的好处是：

- 复用简单
- 后面如果 CLI 写面也要接审计，可以直接复用
- 以后要改格式、改路径、改权限策略，只改一个模块

## 2. Rust 版 WAL 这次故意做成 palace-local，而不是 home-global

Python 用的是：

```text
~/.mempalace/wal/write_log.jsonl
```

Rust 这次改成：

```text
<palace>/wal/write_log.jsonl
```

这样做的原因是 Rust 版现在明确走“独立 palace 目录、独立数据格式”的路线。

把 WAL 也放到 palace 根下，会更符合 Rust 版的隔离原则：

- 不同 palace 的审计日志不会混在一起
- 复制 / 备份一个 palace 时，审计轨迹也跟着走

# 补充知识

## 1. best-effort 安全设置很适合本地工具

这次权限设置没有做成“失败就中断主流程”，而是：

- 能设权限就设
- 失败就继续

这是本地工具里很常见的折中。

因为真正重要的是：

- 写操作不能因为 chmod 失败而完全不可用

但只要平台支持，还是尽量把默认权限收紧。

## 2. JSONL 很适合审计轨迹

JSONL 的好处是：

- 一行一条记录
- append 非常便宜
- 出问题时可以直接 `tail -n`
- 测试也很好写

相比先做一张额外 SQLite 审计表，JSONL 对这种“先把轨迹记下来”场景更轻。

# 验证

这次实际跑过：

```bash
cd rust
cargo fmt
cargo check
cargo test --test mcp_integration
```

重点新增验证：

- `mcp_write_tools_append_palace_local_wal_entries`

# 未覆盖项

这次还没有做这些：

- WAL 的回放 / rollback 工具
- CLI 写命令的审计接入
- 对审计日志做轮转、压缩或清理策略
- 把执行结果 `result` 也系统性写回 WAL；这次先对齐“写前审计”主路径
