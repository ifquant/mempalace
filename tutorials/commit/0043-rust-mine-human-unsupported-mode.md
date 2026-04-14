# 背景

Rust 版 `mine` 已经支持 `--human`，但如果用户传了当前还不支持的 `--mode convos`，CLI 仍然只会输出 JSON 错误块。  
这对脚本友好，但对终端用户来说有点突兀。

# 主要目标

- 让 `mine --human` 在 unsupported mode 场景下也保持人类可读输出
- 保持默认 unsupported mode 仍然是 JSON，避免破坏程序化接口

# 改动概览

- `print_unsupported_mine_mode()` 现在接收 `human` 开关
- `mine --human --mode convos` 会输出：
  - `MemPalace Mine`
  - `Project / Mode / Extract`
  - 当前 Rust 还不支持 conversation/general extraction
  - 下一步建议：改回 `--mode projects`，或者继续用 Python CLI 做 conversation mining
- 默认 `mine --mode convos` 的 JSON 错误保持不变

# 关键知识

- 同一个失败路径完全可以同时维护两种 surface：
  - JSON 给程序
  - human text 给人
- 只要路由点足够靠近 CLI，就不需要改 service 层，也不会污染底层模型结构。

# 补充知识

- `std::process::exit(2)` 不妨碍你先把清晰的人类文本打印到 `stdout`；测试里照样可以断言退出码和输出内容。
- 对“还没实现”的功能，最差的体验不是失败，而是失败时不给下一步建议。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --test cli_integration cli_mine_rejects_unsupported_convos_mode_with_json_hint
cargo test --test cli_integration cli_mine_human_rejects_unsupported_convos_mode_with_text_hint
```

# 未覆盖项

- 这次没有实现 `convos` 或 `general` 真正的 Rust 挖掘逻辑
- 这次没有改 MCP
- 这次没有改 Python CLI
