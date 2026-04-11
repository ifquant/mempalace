# 0022 Rust `mine` 结果补上 room 汇总

这次继续把 Rust 项目 `mine` 的结果往 Python `miner.py` 靠。

## 做了什么

- `MineSummary` 新增：
  - `room_counts`
  - `next_hint`
- `room_counts` 记录每个 room 里本次被处理的文件数
- `next_hint` 固定给出下一步搜索提示：

```text
mempalace search "what you're looking for"
```

## 为什么这样做

Python 版 `mine()` 在结束时会输出两类很重要的信息：

1. 按 room 的分布
2. 接下来该做什么

Rust 版如果只给总数，虽然机器能跑，但人和脚本都不够容易判断：

- 这次到底进了哪些 room
- room 路由是不是偏了
- 挖完之后该走什么下一步

所以这里不去复制 Python 的终端排版，而是把这些信息变成稳定 JSON 字段。这样 CLI、MCP、测试和以后可能的 UI 都能复用。

## 测试

跑了这些验证：

```bash
cd rust && cargo fmt --check
cd rust && cargo test
cd rust && cargo clippy --all-targets --all-features -- -D warnings
```

新增覆盖点：

- `mine_respects_project_config_room_detection_and_scan_rules`
- `mine_can_force_include_gitignored_paths`
- `mine_dry_run_reports_work_without_writing_drawers`
- `cli_init_status_mine_search_round_trip`
- `cli_mine_dry_run_reports_preview_without_writing_drawers`

## 新手知识点

命令行工具如果只输出给人看的一段文本，短期很快，但后面经常会卡住：

- 自动化不好接
- 测试断言很脆
- MCP/UI 还得重新拼一遍

更稳的方式通常是：

1. 先把真正重要的信息做成稳定结构
2. 再决定终端要不要额外排版

Rust 版这次就是沿着这个方向继续收紧。
