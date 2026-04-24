# 背景

Rust 版已经给 `doctor` 和 `prepare-embedding` 做了 `--human` 输出，但失败场景下仍然偏“字段罗列”。  
这对机器可读没问题，但对人排查 embedding 环境并不够直接，尤其是首次模型下载失败时。

# 主要目标

- 让 `doctor --human` 在 warm-up 失败时直接给出更明确的诊断结论
- 让 `prepare-embedding --human` 在失败时直接给出下一步动作建议
- 用稳定的单元测试锁住这些 failure-path 文案

# 改动概览

- 把 `print_doctor_human()` 和 `print_prepare_embedding_human()` 收成可测试的 formatter
- `doctor --human` 现在会总结：
  - model cache 目录是否还没准备好
  - snapshot 存在但 `onnx/model.onnx` 缺失
  - 或者模型文件已经看起来 ready
- warm-up 失败时，输出会增加 `Suggested next step`
- 如果没有显式配置 mirror，会直接建议尝试 `--hf-endpoint https://hf-mirror.com`
- 如果已经配置了 mirror，则提示先验证 mirror 和网络，再重试 `prepare-embedding`

# 关键知识

- 把 CLI 人类输出改成“先 render 成字符串，再 print”更容易测试。  
  这样就不需要在测试里拦 stdout/stderr，只要对字符串断言即可。
- failure-path 文案最好只依赖已经稳定存在的 summary 字段，不要为了写提示再引入新的 schema。  
  这样切片更小，也不会破坏 JSON 协议兼容性。

# 补充知识

- Rust 二进制 crate 也可以直接在 `main.rs` 里写 `#[cfg(test)]` 单元测试，不一定非要单独拆 `lib.rs`。
- 对“依赖外网”的失败场景，优先写纯 formatter 单元测试，比依赖真实坏网络更稳定，也更快。

# 验证

```bash
cd rust
cargo fmt
cargo check
cargo test --bin mempalace-rs doctor_human_failure_suggests_mirror_when_default_endpoint_fails
cargo test --bin mempalace-rs prepare_embedding_human_failure_mentions_configured_mirror_when_present
```

# 未覆盖项

- 这次没有改 `doctor` / `prepare-embedding` 的 JSON 输出
- 这次没有增加新的 summary 字段
- 这次没有碰 `python/` 的现有 CLI 或 MCP 实现
