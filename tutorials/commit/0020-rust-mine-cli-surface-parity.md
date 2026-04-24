# 0020 Rust `mine` CLI 参数面对齐

这次继续把 Rust 版 `mempalace mine` 往 Python 版靠，但刻意只做低风险的 CLI 表面对齐，不直接跳进完整 conversation ingest。

## 做了什么

- Rust `mine` 新增并公开这些参数：
  - `--mode`
  - `--agent`
  - `--extract`
- `MineSummary` 现在会返回：
  - `mode`
  - `extract`
  - `agent`
- `projects` 模式继续是真实可用路径
- `convos` / `general` 还没实现，但现在不会静默吞掉参数，而是返回结构化 JSON：
  - `error`
  - `hint`
  - `mode`
  - `extract`
  - `project_path`

## 为什么这样做

Python CLI 的 `mine` 已经是一个多模式入口：

- `projects`
- `convos`
- `convos --extract general`

Rust 版如果继续只暴露极简参数面，会让未来切换和脚本兼容越来越痛。  
但这一步又不适合直接把 conversation miner 一口气搬完，所以这里先做一个更稳的折中：

1. 先把参数和输出 shape 接上
2. 对支持的模式继续真实执行
3. 对未支持的模式明确拒绝，并给出机器可读响应

这样调用方至少能知道 Rust 版“听得懂这个参数，但能力还没落地到这里”。

## 测试

跑了这些验证：

```bash
cd rust && cargo fmt --check
cd rust && cargo test
cd rust && cargo clippy --all-targets --all-features -- -D warnings
```

新增覆盖：

- `cli_mine_help_mentions_mode_agent_and_extract`
- `cli_mine_rejects_unsupported_convos_mode_with_json_hint`

## 新手知识点

做兼容迁移时，一个很常见的错误是“参数还没实现，就先不暴露”。  
短期看这很安全，但长期会把兼容差距藏起来，导致：

- 文档和代码面不一致
- 调用方不知道功能是“拼错了”还是“还没做”
- 后面很难逐步补齐

更稳的做法是：

- 能支持的参数先接进来
- 暂不支持的分支返回明确、结构化、可测试的失败

这样既不会假装兼容，也不会把兼容面拖成隐性债务。
