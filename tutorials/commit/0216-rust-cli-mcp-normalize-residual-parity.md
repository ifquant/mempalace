# Commit 0216: Rust CLI MCP normalize residual parity

## 背景

Task 5 要收掉 Rust 对 Python 还剩下的一小段 CLI / MCP / normalize 行为差异。

这次对照计划里的 residual parity 清单后，剩下的三个点都不大，但都属于用户实际会碰到的边界行为：

- MCP `initialize` 没带 `protocolVersion` 时，Rust 默认协商版本不对
- CLI `mcp` 在自定义 `--palace ~/...` 路径时，没有像 Python 一样先展开 `~`
- `normalize` 遇到未知 schema 的 `.json` 或坏掉的 `.jsonl` 时，Rust 直接返回 `None`，而不是退回原始文本

这些差异如果不补，CLI/MCP 兼容面会留下明显“不是同一产品”的感觉。

## 主要目标

- 让 MCP initialize 缺失版本时回退到最旧支持协议
- 让 CLI `mcp --palace ~/...` 像 Python 一样做 home 路径展开
- 让 normalize 对未知或 malformed 的 `.json` / `.jsonl` 回退到 raw 内容
- 用测试把这三个 residual parity 行为固定住

## 改动概览

- 更新 `rust/src/mcp_schema_support.rs`
- 更新 `rust/src/config.rs`
- 更新 `rust/src/normalize.rs`
- 新增 `rust/tests/mcp_integration.rs` 的 initialize fallback 测试
- 新增 `rust/tests/cli_integration.rs` 的 MCP tilde path 测试
- 在 `rust/src/normalize.rs` 内新增 unknown JSON 和 malformed JSONL raw fallback 测试

## 关键知识

### 1. MCP 协议默认值要偏保守，不要偏最新

当客户端没有显式声明 `protocolVersion` 时，服务端默认回最旧支持版本更稳妥，因为这代表“尽量兼容最宽的调用方”，而不是假设对方一定支持较新的协议日期。

这次把 `negotiate_protocol(None)` 从 `SUPPORTED_PROTOCOL_VERSIONS[1]` 改成数组最后一个版本，也就是当前支持列表里的最旧协议。

### 2. CLI 路径显示和真实解析都要先过同一层规范化

`--palace ~/tmp/my palace` 这种参数如果不先扩展 `~`，Rust 会把它当成普通相对路径，再拼到当前工作目录下面，最终：

- 实际运行路径错了
- MCP setup 输出也会把错路径展示给用户

这次把 `normalize_path()` 补成先识别 `~` / `~/...`，再决定是直接返回绝对路径还是拼接当前目录。

### 3. normalize 的格式识别失败，不等于文本没有价值

Python 这里的策略更实用：如果一个文件扩展名是 `.json` 或 `.jsonl`，但内容既不是支持的 export schema，也不是可恢复的 JSONL transcript，就直接把原文返回。

这样做的好处是：

- CLI `normalize` 仍然能给出稳定结果
- MCP `mempalace_normalize` 不会把未知导出格式直接判成“不支持”
- 用户至少还能拿到 raw 内容继续排查

## 补充知识

### 1. `Option::None` 在 normalize 路径里通常代表“彻底不支持”

如果只是“识别失败但文本还值得保留”，返回 `Some(raw.to_string())` 往往更符合工具型产品的容错预期。真正该返回 `None` 的情况应该更接近“这个输入不该继续往下走”。

### 2. CLI integration 里验证 `~` 展开，最好显式控制 `HOME`

这次测试没有依赖开发机真实 home 目录，而是通过临时目录覆盖 `HOME`，这样断言更稳定，也不会把个人机器路径耦合进测试。

## 验证

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo test --test mcp_integration mcp_initialize_missing_version_uses_oldest_supported_protocol --quiet
cargo test --test cli_integration cli_mcp_custom_palace_expands_tilde_like_python --quiet
cargo test normalize_json_file_with_unknown_schema_falls_back_to_raw_like_python --quiet
cargo test normalize_malformed_jsonl_falls_back_to_raw_like_python --quiet
```

## 未覆盖项

- 这次没有修改 `python/uv.lock`
- 这次没有修改 `docs/superpowers/`
- 这次没有修改 `docs/rust-python-deep-gap-audit.md` 或 `docs/rust-python-deep-gap-list.md`
- 这次没有改 KG、registry、maintenance、layer 相关实现
- 这次没有扩展 `service_integration.rs`
