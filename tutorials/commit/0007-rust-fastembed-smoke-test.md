# 背景

前几次提交已经把 Rust 版 embedding 路径逐步打通了：

- 有 `fastembed` provider
- 有 `doctor`
- 有 `prepare-embedding`
- 还能通过 `hf-mirror` 完成真实 warm-up

但还差最后一个很关键的点：  
仓库里的自动化验证还没有真正覆盖：

- `prepare-embedding -> mine -> search`

也就是说，我们知道“模型能准备好”，但还没有在测试里证明“准备好之后，整条 CLI 主链路真的能工作”。

# 主要目标

这次提交的目标是补上一条真实但可控的 `fastembed` 端到端 smoke test：

1. 覆盖 `prepare-embedding`
2. 覆盖 `mine`
3. 覆盖 `search`
4. 保证它不会拖慢普通 `cargo test`

# 改动概览

主要改动如下：

- `rust/tests/cli_integration.rs`
  - 新增 `cli_fastembed_prepare_mine_search_smoke`
  - 测试默认标记为 `#[ignore]`
  - 测试里会：
    - 创建临时项目
    - 调 `prepare-embedding`
    - 调 `mine`
    - 调 `search`
    - 检查搜索结果里是否命中预期语义文本
  - 抽出 `run_cli_json()` 辅助函数，统一做 CLI 调用和 JSON 解析
- `rust/README.md`
  - 补充如何显式运行这条 ignored smoke test
  - 说明什么时候需要设置 `MEMPALACE_RS_TEST_HF_ENDPOINT`

# 关键知识

## 1. 真实集成测试不一定适合放进默认测试集

这条测试为什么默认 `ignored`？

因为它依赖：

- 本地 `onnxruntime`
- `fastembed` 真实初始化
- 模型缓存状态
- 某些环境下还需要 HuggingFace mirror

如果把这类测试直接塞进普通 `cargo test`，本地开发和 CI 都会变得脆弱。  
更好的做法是：

- 默认测试只保留稳定、快速、离线的验证
- 把真实外部依赖 smoke test 设计成显式执行

这样既保住反馈速度，也保住真实回归能力。

## 2. CLI smoke test 最好直接断言结构化 JSON

这次没有只用字符串包含判断，而是把命令输出解析成 `serde_json::Value` 再断言字段。

这样做的好处是：

- 不容易被格式细节误伤
- 更接近真实 API 契约
- 后面如果扩展字段，也更容易局部校验

# 补充知识

## 为什么这里仍然保留 `hash` 版本的默认集成测试

因为 `hash` provider 的价值不是检索质量，而是：

- 完全离线
- 稳定
- 快

它非常适合做“结构正确性”和“主链路没有坏掉”的默认回归。  
而 `fastembed` smoke test 则负责验证：

- 真实模型路径
- 真实向量写入
- 真实语义检索闭环

两类测试不是互相替代，而是分工不同。

## 为什么用环境变量传测试镜像地址

这里没有把镜像地址写死进测试代码，而是使用：

- `MEMPALACE_RS_TEST_HF_ENDPOINT`

这样做的好处是：

- 测试可以在不同网络环境复用
- 默认情况下不强行绑定某一个镜像
- 用户本机和未来 CI 都能自行选择下载路径

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && MEMPALACE_RS_TEST_HF_ENDPOINT=https://hf-mirror.com cargo test cli_fastembed_prepare_mine_search_smoke -- --ignored --nocapture`

真实结果：

- 默认 `cargo test` 通过
- ignored smoke test 也通过
- 已确认 `prepare-embedding -> mine -> search` 在真实 `fastembed` 路径下可用

# 未覆盖项

这次没有继续做：

- 把这条 smoke test 接入 CI
- 对照 Python 版进一步收紧 taxonomy / chunking / ignore 语义
- 为 `fastembed` 路径补 benchmark 基线
- 做 `repair/migrate` 与 schema versioning

所以这次提交的定位是：  
先给 Rust 版补上一条真实可执行的 `fastembed` 端到端回归，而不是继续扩散功能面。
