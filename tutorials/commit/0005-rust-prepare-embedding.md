# 背景

上一提交已经有了 `doctor`，可以知道 fastembed 为什么没跑起来。

但只有诊断还不够，因为实际工作流还缺一条正式命令：

- 先准备模型
- 再跑 `mine/search`

如果用户每次都要手工反复跑 `doctor --warm-embedding`，体验仍然很差。

# 主要目标

这次提交补了一条正式的 embedding 准备路径：

1. 新增 `prepare-embedding` CLI 命令
2. 支持重试与等待间隔
3. 把 `doctor` 输出细化到模型文件级别

# 改动概览

主要改动：

- `rust/src/model.rs`
  - 新增 `PrepareEmbeddingSummary`
  - `DoctorSummary` 增加：
    - `expected_model_file`
    - `expected_model_file_present`
    - `hf_endpoint`
- `rust/src/embed.rs`
  - fastembed 记录 `model_file`
  - `doctor` 能给出预期模型文件路径
  - `doctor` 能判断预期模型文件是否真的存在
- `rust/src/service.rs`
  - 新增 `prepare_embedding(attempts, wait_ms)`
- `rust/src/main.rs`
  - 新增 `prepare-embedding` 命令
- `rust/tests/cli_integration.rs`
  - 覆盖 hash provider 下的 `prepare-embedding`
- `rust/README.md`
  - 增加推荐首次运行流程

# 关键知识

## 1. “模型目录存在”不等于“模型文件可用”

这次最关键的收敛点是把问题从：

- “cache 目录可能不对”

推进成：

- “预期文件 `snapshots/onnx/model.onnx` 不存在”

这两者的信息价值差很多。  
只有定位到文件级，后面才知道是：

- 下载未完成
- snapshot 没写出来
- HF endpoint 不通
- 模型 repo 结构不匹配

## 2. 重试命令比临时 shell 更适合做正式工作流

`prepare-embedding` 的价值不是它做了特别复杂的事，而是它把一件经常要手工做的事变成了正式接口：

- 有固定参数
- 有固定返回 JSON
- 有固定失败语义

后面无论接 smoke script、CI，还是 MCP，只要复用这条命令就行。

# 补充知识

## 为什么 `prepare-embedding` 仍然可能失败

因为这条命令负责的是“尝试准备并把结果说清楚”，不是强行保证网络成功。

这轮真实验证里，返回结果已经稳定到：

- provider: `fastembed`
- model: `MultilingualE5Small`
- ORT dylib 已找到
- 缺失文件：`snapshots/onnx/model.onnx`

这说明工程问题已经从“黑盒初始化失败”收敛成“模型文件还没取到”。

## 为什么这一步值得单独提交

因为它把 Rust 版 embedding 工作流从：

- 只能 debug

推进到了：

- 可以准备
- 可以重试
- 可以输出结构化状态

这是一条完整的能力切片，后面做自动下载优化、镜像支持、下载回退时都能直接接着演进。

# 验证

已完成：

- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run -- --palace /tmp/... prepare-embedding --attempts 2 --wait-ms 100`

真实结果：

- 命令成功返回结构化 JSON
- 当前失败原因收敛为：
  - `Failed to retrieve onnx/model.onnx`
- `doctor.expected_model_file` 已能直接指出目标文件路径

# 未覆盖项

这次仍然没有做：

- 自动切换 HuggingFace 镜像
- 下载成功后的端到端 fastembed `mine/search` 回归测试
- 自动修复损坏 snapshot
- 多模型批量预取

所以这次提交的定位是：  
把 embedding 准备动作变成一个正式命令，并把失败定位推进到具体模型文件。
