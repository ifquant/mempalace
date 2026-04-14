# 背景

`mine --human` 已经能输出人类可读摘要，但当项目里没有任何可挖掘文件时，终端上只会看到一串 `0`。  
这对脚本没问题，但对人来说不够直观。

# 主要目标

- 让 `mine --human` 在空项目或全被过滤时明确提示“没有匹配文件”
- 保持 JSON 输出和底层 `MineSummary` 完全不变

# 改动概览

- 在 `print_mine_human()` 里增加 `files_planned == 0` 分支
- 人类输出现在会额外显示：
  - `No matching files found.`
  - `Check your project path, ignore rules, and supported file types.`
- 新增 CLI 回归，覆盖空项目 human 输出

# 关键知识

- 这类“纯展示层增强”优先放在 CLI formatter，而不是改 summary schema。  
  这样切片更小，也不会影响 MCP、JSON 或测试夹具。
- 当统计数字全为 0 时，补一句结论性文本，往往比再加更多字段更有用。

# 补充知识

- 很多 CLI 易用性问题并不是“功能没做”，而是“失败或空结果时没把结论说出来”。
- 用一个只含 `target/generated.bin` 的目录当 fixture，很适合稳定覆盖“扫描后无匹配文件”的场景。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_mine_human_empty_project_reports_no_matching_files
```

# 未覆盖项

- 这次没有改 `mine` 的 JSON 输出
- 这次没有改 `MineSummary` 结构
- 这次没有改变扫描规则本身
