# 背景

前面的 Rust 版本已经有了 `mine` 主链路，但它的 project miner 语义还比较粗：

- `room` 主要来自目录名
- 扫描规则偏宽
- 不认识 `mempalace.yaml`

这和 Python 版的真实行为差了一截。  
如果继续在这个基础上堆功能，后面 CLI 和 MCP 虽然“能跑”，但搜出来的分类会越来越偏。

# 主要目标

这次提交的目标不是扩功能面，而是把 Rust project miner 向 Python 版再拉近一段：

1. 支持读取 `mempalace.yaml` / `mempal.yaml`
2. 用 Python 风格的 room 路由规则替代“只看父目录”
3. 收紧文件扫描规则
4. 覆盖 `.gitignore + include_ignored` 的关键行为

# 改动概览

主要改动如下：

- `rust/src/service.rs`
  - 新增 project config 读取：
    - `mempalace.yaml`
    - `mempal.yaml`
  - 支持从配置中读取：
    - `wing`
    - `rooms`
  - `detect_room()` 改成更接近 Python 的三段式路由：
    - 路径片段匹配
    - 文件名匹配
    - 内容关键词打分
  - `discover_files()` 增加扫描约束：
    - 跳过常见缓存/生成目录
    - 只扫可读扩展名
    - 跳过固定文件名
    - 跳过超大文件
    - 支持显式 force-include 精确路径
  - `chunk_text()` 改成更接近 Python miner 的：
    - `800` 字符块
    - `100` 字符 overlap
    - 优先在换行或段落边界切
    - 跳过小于 `50` 字符的碎片
- `rust/tests/service_integration.rs`
  - 新增 project config + room 路由测试
  - 新增 `.gitignore + include_ignored` 测试
- `rust/README.md`
  - 补充当前 Rust project miner 的行为说明

# 关键知识

## 1. 项目挖掘最容易“看起来能用，实际上分类很偏”

如果 miner 只是把文件按目录塞进向量库，表面上：

- `mine` 成功了
- `search` 也能返回结果

但实际问题是：

- room 分类会偏
- taxonomy 会失真
- 用户后续用 `--room` 过滤时体验会很差

所以这次最重要的不是“多扫几个文件”，而是把分类语义往 Python 版靠。

## 2. `.gitignore` 对齐不能只靠“能忽略”

Python 版 project miner 有两个同时存在的能力：

- 默认尊重 `.gitignore`
- 用户仍然可以用 `--include-ignored` 强行带回特定路径

如果 Rust 只做前者，不做后者，真实项目里会很难用，因为很多有价值的文档会被统一放进被忽略目录。  
这也是为什么这次专门补了 force-include 测试。

# 补充知识

## 为什么没有强行要求 `mempalace.yaml` 必须存在

Python 版 project miner 默认要求先 `init`，然后依赖配置文件。  
Rust 这阶段还在重写中，所以这里保留了更平滑的 fallback：

- 有配置文件时按配置走
- 没有配置文件时退回默认 `wing` 和 `general` room

这样更适合重写阶段逐步迁移，不会因为仓库里暂时没配置就完全没法试。

## 为什么扫描规则要先收紧，再谈性能

扫描范围过宽会直接带来两个问题：

- 检索质量下降，因为缓存和噪音文件被误挖进去
- 性能指标失真，因为 miner 在做很多没价值的 IO

所以在 benchmark 之前先把“扫什么”定得更准，通常比提前微优化更值。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

这轮新增验证点：

- `mine_respects_project_config_room_detection_and_scan_rules`
- `mine_can_force_include_gitignored_paths`

验证结果：

- Rust miner 现在会优先使用 `mempalace.yaml` 的 `wing/rooms`
- room 分类不再只是父目录名
- 默认扫描范围更接近 Python 版
- `.gitignore` 忽略和显式 include 能同时成立

# 未覆盖项

这次没有继续做：

- Python `init` 的 room/entity 自动探测
- conversation mining / `--mode convos`
- Python 搜索输出和 MCP 返回字段的进一步逐字兼容
- schema version / migrate / repair

所以这次提交的定位是：  
先把 Rust 的 project miner 从“能跑”推进到“分类和扫描规则更接近 Python 版”。
