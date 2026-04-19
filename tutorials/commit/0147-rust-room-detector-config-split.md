# 背景

`rust/src/room_detector.rs` 之前已经是一个独立模块了，但它内部仍然把两类不同职责放在一起：

- 读取 `mempalace.yaml` / `mempal.yaml`
- 根据目录、文件名、内容关键词做 room detection

这两类逻辑都和 room 有关，但关注点不一样：

- config 读取更偏文件 IO 和配置语义
- detection 更偏启发式分类规则

如果继续把它们放在一个文件里，后面无论是改配置格式还是改 room heuristic，都会把同一个文件越改越大。

# 主要目标

这次提交的目标是把 `room_detector.rs` 内部拆成：

- config/load 一层
- detection/heuristic 一层
- facade 一层

同时保持外部 `crate::room_detector::*` surface 不变。

# 改动概览

这次新增了两个内部模块：

- `rust/src/room_detector_config.rs`
- `rust/src/room_detector_detect.rs`

拆分后的职责边界是：

## `room_detector_config`

负责：

- `load_project_config()`
- `load_project_rooms()`

也就是从项目目录里找到 `mempalace.yaml` / `mempal.yaml`，并把配置反序列化成 `ProjectConfig` / `ProjectRoom`。

## `room_detector_detect`

负责：

- `detect_rooms()`
- `detect_room()`
- folder/file heuristic map
- `normalize_roomish()`

也就是房间探测本身，包括：

- 从目录结构推断 room
- 从文件名和内容关键词做 room routing
- 在没有显式 config 时生成默认 room 集合

## `room_detector`

现在只保留：

- `ProjectConfig`
- `ProjectRoom`
- `RoomDetection`
- public re-export
- 对应测试入口

这样 facade 继续稳定，但具体实现已经按职责分开。

# 关键知识

## 1. 配置读取和启发式检测虽然相关，但不是同一类变化

很多模块一开始会因为主题相同而被放到一起，比如“都和 room 有关”。

但工程上更重要的是看“变化原因”：

- config 读取会随着文件格式、默认值策略变化
- detection heuristic 会随着产品经验和数据分布变化

变化原因不同，就值得拆开。

## 2. facade re-export 能保住上层调用面

这次 `bootstrap.rs` 和 `miner_project.rs` 继续直接用：

- `detect_rooms()`
- `load_project_config()`
- `load_project_rooms()`
- `detect_room()`

调用方式没有变，因为 `room_detector.rs` 把这些入口重新 re-export 回来了。

这让我们可以只优化内部结构，而不必同步改所有调用点。

# 补充知识

## 1. 先拆“文件 IO vs 规则逻辑”，通常是低风险高收益的一刀

很多业务模块继续细拆时，最稳的一刀往往不是追求最细颗粒度，而是先把：

- 文件 IO / 配置解析
- 规则 / 算法 / 评分逻辑

分开。

因为这两类代码在阅读和维护上本来就属于不同脑区。

## 2. 测试锚点留在 facade，有利于保持入口视角

这次没有把 `room_detector` 的测试拆散到子文件，而是继续让 facade 带着这两个核心测试：

- `detect_rooms_prefers_folder_structure_and_falls_back_to_general`
- `detect_room_uses_path_and_keyword_rules`

这样维护者打开顶层 facade 时，仍然能一眼看到这条能力线最重要的行为约束。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

这次没有改这些内容：

- 没有改变 room detection 的外部行为
- 没有改变 `mempalace.yaml` 的格式
- 没有改变 bootstrap / mining 对 room detector 的调用方式
- 没有继续拆 `room_detector_detect` 里的 folder map 和 scoring 细节
- 没有改 Python `room_detector_local.py`
