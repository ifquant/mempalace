# 背景

上一提交已经把 Rust 版 MemPalace 接到了真实的本地 embedding provider：`fastembed`。

但工程上还缺一个很实际的问题答案：

- 为什么 fastembed 还没跑起来？
- 是 ONNX Runtime 动态库没装？
- 还是模型没下载完？
- 还是 provider 配置错了？

如果没有一个稳定的诊断入口，每次都要靠手工 shell 命令和临时脚本排查，成本很高。

# 主要目标

这次提交的目标很单一：

1. 给 Rust CLI 增加 `doctor` 命令
2. 让 embedding/runtime 的可观测性进入正式接口
3. 支持可选的 `--warm-embedding`，用于真实触发一次 provider 初始化

# 改动概览

主要改动：

- `rust/src/model.rs`
  - 新增 `DoctorSummary`
- `rust/src/embed.rs`
  - `EmbeddingProvider` trait 新增 `doctor()`
  - `HashEmbedder` 提供简单健康报告
  - `FastEmbedder` 报告：
    - provider / model / dimension
    - cache dir
    - HuggingFace model cache dir
    - model cache 是否已完整存在
    - ONNX Runtime 动态库路径
    - 可选 warm-up 的结果和错误信息
- `rust/src/service.rs`
  - 新增 `doctor()` service
- `rust/src/main.rs`
  - 新增 `doctor` CLI 子命令
- `rust/tests/cli_integration.rs`
  - 覆盖 hash provider 下的 `doctor`
- `rust/README.md`
  - 补充 `doctor` 使用说明

# 关键知识

## 1. “能启动 provider”和“能真正生成 embedding”不是一回事

本地 embedding 链路至少有三层依赖：

- provider 配置正确
- ONNX Runtime 动态库存在
- 模型文件完整缓存

只检查其中一层是不够的。  
这次 `doctor` 把三层状态拆开了，所以可以更快知道问题卡在哪。

## 2. 诊断命令最好返回结构化 JSON

如果 `doctor` 只是打印一句“看起来没问题”，后面自动化流程、CI、或者脚本就没法复用。

现在它直接返回结构化 JSON，后面可以很容易拿去做：

- smoke script
- CI artifact
- MCP 工具扩展
- 故障排查文档

# 补充知识

## 为什么 `model_cache_present=false` 很重要

在 fastembed 的首次运行里，最常见的不是程序代码错，而是模型文件还在下载中，或者只留下了 `.part` / `.lock` 文件。

如果不把这个状态显式暴露出来，用户很容易误以为：

- ORT 装坏了
- fastembed 不兼容
- LanceDB 有问题

实际上只是模型缓存还没准备好。

## 为什么 `--warm-embedding` 仍然可能失败

`doctor --warm-embedding` 的目标不是保证成功，而是把失败原因稳定收敛出来。

比如这次实际验证就能拿到这种错误：

- `Failed to retrieve onnx/model.onnx`

这就说明：

- provider 代码路径已经走到真实模型初始化
- ORT 动态库没卡住
- 问题落在模型文件准备这一层

# 验证

已完成：

- `cd rust && cargo check`
- `cd rust && cargo test`
- `cd rust && cargo fmt`
- `cd rust && cargo fmt --check`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

额外验证：

- `cargo run -- --palace /tmp/... doctor`
  - 成功返回 fastembed/provider/cache/ORT 路径诊断 JSON
- `cargo run -- --palace /tmp/... doctor --warm-embedding`
  - 成功返回 warm-up 失败原因：`Failed to retrieve onnx/model.onnx`

这说明诊断链路已经能稳定区分：

- 运行时库路径问题
- 模型缓存缺失问题
- provider 初始化问题

# 未覆盖项

这次仍然没有做：

- 自动下载并补齐模型缓存
- `doctor` 的 MCP 暴露
- 对更多 fastembed 模型的诊断适配
- provider 级 benchmark
- download retry / backoff 策略

所以这次提交的定位是：  
给 Rust 版 MemPalace 增加一个正式的 embedding/runtime 健康检查入口，而不是替你自动修复所有模型准备问题。
