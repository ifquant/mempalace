# 背景

上一提交已经把 `prepare-embedding` 做成了正式命令，但真实运行里还有一个卡点：

- 默认 HuggingFace 下载路径在当前环境下不稳定
- `fastembed` 初始化会报 `Failed to retrieve onnx/model.onnx`

这说明问题已经不在 ORT 动态库，而在模型获取路径。  
如果这一步不解决，Rust 版虽然“接口齐了”，但第一次真实使用仍然会卡住。

# 主要目标

这次提交的目标很聚焦：

1. 给 Rust CLI 增加显式的 HuggingFace endpoint override
2. 让 `fastembed` 初始化前真正使用这个 override
3. 修正 `doctor` 里“预期模型文件路径”的推断逻辑
4. 用真实命令验证镜像路径下的 warm-up 能成功

# 改动概览

主要改动如下：

- `rust/src/config.rs`
  - `EmbeddingSettings` 新增 `hf_endpoint`
  - 支持读取 `MEMPALACE_RS_HF_ENDPOINT`
  - 如果没设置仓库私有变量，则回退读取 `HF_ENDPOINT`
- `rust/src/main.rs`
  - CLI 全局新增 `--hf-endpoint`
  - 所有会构造 `AppConfig` 的命令都会统一套用 CLI override
- `rust/src/embed.rs`
  - `FastEmbedder` 持有 `hf_endpoint`
  - 初始化 `TextEmbedding` 前会设置 `HF_ENDPOINT`
  - `doctor` 返回时会优先回显当前实际使用的 endpoint
  - `expected_model_file` 不再硬编码猜测 `snapshots/onnx/...`
  - 改为读取 `refs/main`，再定位真实 snapshot 下的 `onnx/model.onnx`
- `rust/README.md`
  - 文档加入镜像变量和 `--hf-endpoint` 示例
  - 推荐首次运行流程改成带镜像的准备命令

# 关键知识

## 1. HuggingFace cache 不是固定单层目录

这次一个关键修正是：

- 不能把目标文件简单拼成 `snapshots/onnx/model.onnx`

HuggingFace 本地缓存通常会先写：

- `refs/main`

它里面保存当前分支实际指向的 snapshot hash。  
真正的模型文件路径通常是：

- `snapshots/<hash>/onnx/model.onnx`

所以如果想做可靠诊断，应该先读 `refs/main`，再解析到真实 snapshot。

## 2. “支持配置”不等于“运行时真的生效”

很多工程里会犯一个常见错误：

- 配置层已经接了新字段
- 但真正初始化第三方库前，没有把变量注入到运行环境

这次 `hf_endpoint` 的关键不是把字段加进 `AppConfig`，而是：

- 在 `fastembed` 实际创建 `TextEmbedding` 之前调用 `configure_hf_endpoint()`

只有这样，第三方下载逻辑才会真正走镜像地址。

# 补充知识

## 为什么同时保留环境变量和 CLI 参数

两种入口适合不同场景：

- 环境变量适合长期本机默认配置
- CLI 参数适合单次试验、脚本 smoke test、以及排查问题时快速覆盖

这也是为什么这次不是只加 `MEMPALACE_RS_HF_ENDPOINT`，还额外加了全局 `--hf-endpoint`。

## 为什么这次值得单独提交

因为它不是一个“顺手小修”，而是把 Rust 版 embedding 路径从：

- 可以诊断失败

推进到了：

- 可以通过镜像显式修正下载路径
- 可以真实 warm-up 成功
- 可以在诊断输出里定位到准确模型文件

这已经是一个完整的能力闭环。

# 验证

已完成：

- `cd rust && cargo check`
- `cd rust && cargo test`
- `cd rust && cargo fmt --check`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run --quiet -- --palace /tmp/... --hf-endpoint https://hf-mirror.com prepare-embedding --attempts 1 --wait-ms 0`

真实结果：

- `prepare-embedding` 返回 `success: true`
- `warmup_ok: true`
- `hf_endpoint` 为 `https://hf-mirror.com`
- `expected_model_file_present: true`
- ORT 动态库路径识别为 `/opt/homebrew/opt/onnxruntime/lib/libonnxruntime.dylib`

# 未覆盖项

这次没有继续做：

- 下载完成后的完整 `mine/search` fastembed 端到端回归
- 自动回退多个镜像 endpoint
- snapshot 损坏后的自动修复
- 把 `hf_endpoint` 暴露到 MCP 参数层

所以这次提交的定位是：  
先把 fastembed 的真实模型准备路径打通，并把镜像覆盖能力和文件级诊断做扎实。
