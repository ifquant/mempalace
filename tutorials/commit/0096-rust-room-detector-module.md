## 背景

前一轮刚把 Rust 的 `entity_detector` 从 `bootstrap.rs` 里拆出来，但 room 这条线还处在旧状态：

- `bootstrap.rs` 里有一套 `detect_rooms()`
- `service.rs` 里又有一套 `load_project_rooms()` / `detect_room()`

这意味着 Rust 版虽然已经能：

- `init` 时自动写 `mempalace.yaml`
- `mine` 时按 room 路由文件

但这两条路径背后的 room vocabulary 和 fallback 规则并不是一个公共模块。继续这样走，后面很容易出现：

- bootstrap 检测到的 room 和 miner 理解的 room 漂移
- Python `room_detector_local.py` 的对齐点散在两处

所以这一轮要把 room 检测也收成一个正式模块。

## 主要目标

- 给 Rust 新增独立 `room_detector` 模块
- 把 room bootstrap、配置读取、项目 room 路由都搬进去
- 让 `bootstrap` 和 `service` 共用同一套 room 规则
- 把这层库 API 写进 README

## 改动概览

- 新增 [rust/src/room_detector.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/room_detector.rs)
  - `ProjectConfig`
  - `ProjectRoom`
  - `RoomDetection`
  - `detect_rooms()`
  - `load_project_config()`
  - `load_project_rooms()`
  - `detect_room()`
- 更新 [rust/src/lib.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/lib.rs)
  - 导出 `room_detector`
- 更新 [rust/src/bootstrap.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/bootstrap.rs)
  - 删除内嵌 room detection 逻辑，改用新模块
- 更新 [rust/src/service.rs](/Users/dev/workspace2/agents_research/mempalace/rust/src/service.rs)
  - 删除内嵌 project room config / detect_room 逻辑，改用新模块
- 更新 [rust/README.md](/Users/dev/workspace2/agents_research/mempalace/rust/README.md)

## 关键知识

### 1. room bootstrap 和 room routing 应该是同一层能力

如果 `init` 生成 `mempalace.yaml` 时用的是一套规则，而 `mine` 路由文件时又用另一套规则，
那 room 配置最后只会越来越像“偶然对上”，而不是一个稳定契约。

这一轮把两边都收进 `room_detector` 之后，逻辑边界就清楚了：

- `room_detector`
  负责 room vocabulary、检测、配置读取、路由
- `bootstrap`
  负责把 room 检测结果写成项目 bootstrap 文件
- `service`
  负责挖掘时调用 room detector

### 2. 对齐 Python，不只是补命令，还要补模块形状

Python 侧有独立的 `room_detector_local.py`。Rust 如果只在 CLI 上看起来“能 init、能 mine”，
但库层仍然把 room 逻辑散在两个文件里，那么它其实还没有真正形成对齐。

这轮的重点就是把 Rust 的模块 shape 也往 Python 收。

## 补充知识

### 1. 公共模块抽取时，优先把“规则常量”一起搬走

像 room 这种能力，真正容易漂移的往往不是函数名，而是：

- vocabulary
- fallback 条件
- 默认 `general`
- config 读取规则

所以这次不是只搬一个 `detect_room()`，而是把：

- `ProjectRoom`
- `ProjectConfig`
- `RoomDetection`
- room map

一起收进模块，避免“函数共用了，规则还分裂”。

### 2. 服务层测试继续保留是有价值的

虽然 `detect_room()` 被抽出去了，但 service 里的回归没有必要全部搬走。
因为用户真正依赖的是“service 里的 project mining 是否按预期路由”。

模块测试保证局部逻辑，service 测试保证接线没有断，这两层都值得保留。

## 验证

实际运行：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

本次新增覆盖了：

- `room_detector` 能从目录结构生成 room 列表
- `room_detector` 的 `detect_room()` 继续满足原有 service 回归
- 现有 `init` / `mine` / CLI / MCP 回归继续通过

## 未覆盖项

- 这次没有新增独立 `room-detect` CLI，只先把库层模块补出来
- 这次没有把 Python `room_detector_local.py` 的交互确认流迁到 Rust
- 这次没有改 Python 代码，只是继续把 Rust 库层形状往 Python 对齐
