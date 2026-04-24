# 0026 Rust `mine` 结果补上 Python 头部上下文

这次继续把 Rust `mine` 的结构化输出往 Python `miner.py` 靠，但还是坚持 JSON-first，不回退到只给人看的 banner。

## 做了什么

- `MineSummary` 新增：
  - `configured_rooms`
  - `files_planned`

它们对应 Python `mine()` 开头会打印的两条关键信息：

- `Rooms: ...`
- `Files: ...`

## 为什么这样做

Python 版在正式开始处理之前，会先告诉你：

1. 这次按哪些 room 配置来分流
2. 应用 `scan_project()` 和 `--limit` 后，预计会处理多少文件

Rust 之前虽然有：

- `files_seen`
- `files_mined`

但那是“处理之后”的数字，不等价于 Python 开头的头部信息。  
这次把它们补成稳定字段后，CLI、脚本和以后可能的 UI 都能直接消费。

## 验证

通过的最低成本验证：

```bash
cd rust && cargo fmt --check
cd rust && cargo check
```

另外这次只改 summary 字段，没有再动 on-disk schema。

## 新手知识点

命令行工具里很常见的一种信息分层是：

1. 开始前告诉你“这次准备做什么”
2. 过程中告诉你“当前做到哪一步”
3. 结束后告诉你“最后做成了什么”

如果只保留第 3 层，很多自动化和调试场景会缺上下文。  
这次补的 `configured_rooms` 和 `files_planned`，就是把第 1 层也结构化下来。
