# 背景

这一提交把 `mempalace` 的 Rust 重写从“只有依赖和空 crate”推进到“第一阶段最小可用骨架”。

目标不是一次性覆盖 Python 版所有能力，而是先把下面这条主链路打通：

- 本地 palace 初始化
- 项目文件挖掘
- 本地语义检索
- 只读 MCP 工具
- 基础 KG 读写

同时明确一条原则：Rust 版和 Python 版先并行演进，不共享 palace 数据目录。

# 主要目标

这次提交主要完成了 5 件事：

1. 建立 Rust 分层结构：`config / model / storage / service / mcp / cli`
2. 用 `rusqlite + LanceDB` 做本地优先存储骨架
3. 实现 `init / mine / search / status` 四个 CLI 主命令
4. 实现只读 MCP 工具：`mempalace_status`、`mempalace_list_wings`、`mempalace_list_rooms`、`mempalace_get_taxonomy`、`mempalace_search`
5. 补上测试与 benchmark 入口，让后续迭代有验证抓手

# 改动概览

核心代码新增与调整：

- `rust/src/config.rs`
  - 统一 Rust palace 默认目录解析，默认走 `~/.mempalace-rs/palace`
- `rust/src/model.rs`
  - 定义 drawer、search、status、taxonomy、KG triple 等核心模型
- `rust/src/storage/sqlite.rs`
  - 管理 taxonomy、KG、ingest bookkeeping、drawer metadata
- `rust/src/storage/vector.rs`
  - 管理 LanceDB 表创建、按文件替换向量数据、向量检索
- `rust/src/service.rs`
  - 统一编排 `init / mine / search / taxonomy / kg / status`
- `rust/src/mcp.rs`
  - 提供只读 MCP 请求分发与 stdio server
- `rust/src/main.rs`
  - 提供 CLI 入口，并修正 `init` 会真正使用传入的 palace 目录

测试与基准：

- `rust/tests/cli_integration.rs`
  - 覆盖 CLI 主链路
- `rust/tests/mcp_integration.rs`
  - 覆盖只读 MCP 工具
- `rust/tests/service_integration.rs`
  - 覆盖幂等 init、taxonomy、KG 最小闭环
- `rust/benches/ingest_search.rs`
  - 增加 ingest/search benchmark 骨架

# 关键知识

## 1. LanceDB 适合做本地向量层，但它不是轻依赖

只要把 `LanceDB` 拉进来，编译链会把 `arrow / datafusion / lance / tantivy` 一并带进来。  
这意味着两个工程事实：

- 平时开发必须尽量复用增量编译，不要频繁清理 `target/`
- benchmark 的首次冷编译会明显慢于普通 `cargo test`

这不是项目写错了，而是选型带来的真实代价。

## 2. MCP 层最好直接 async，不要在库层硬包同步 runtime

一开始如果把 `tools/call` 写成“同步函数里临时新建 Tokio runtime”，在 `#[tokio::test]` 或其它异步宿主里很容易出现 nested runtime panic。

更稳的方式是：

- `handle_request` 直接做成 async
- stdio server 直接 `await`
- 测试也直接 `await`

这能少掉一整类 runtime 嵌套错误。

# 补充知识

## 为什么 Rust 版不直接读 Python palace

因为第一阶段更重要的是先把：

- 数据模型边界
- 本地存储选型
- 服务层 API
- CLI/MCP 协议面

这些工程地基做稳。  
如果一开始就强绑 Python 现有数据格式，后面重构 storage 和索引布局时会被兼容包袱拖住。

## 为什么 embedding 先用本地确定性哈希方案

这次不是为了追求检索质量上限，而是为了先把：

- mine -> embed -> store -> search

整条链路稳定打通。  
等后续要接入真正模型时，只要替换 embedding provider，就能保留现有 service / storage / surface 边界。

# 验证

已完成：

- `cd rust && cargo check`
- `cd rust && cargo test`
- `cd rust && cargo fmt --check`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

额外情况：

- 已新增 `rust/benches/ingest_search.rs`
- `cd rust && cargo bench --no-run` 会触发非常重的 release 冷编译，首次耗时明显长于前述命令

# 未覆盖项

这次仍然没有做：

- 写入型 MCP 工具
- hooks
- repair / migrate
- AAAK 生成与 wake-up
- conversation mining
- 与 Python palace 数据互通
- 真正语义 embedding 模型接入

所以这次交付的定位很明确：  
它是 Rust 版 MemPalace 第一阶段的“可跑骨架”，不是最终替换版。
