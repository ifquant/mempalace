# 背景

`rust/src/entity_detector.rs` 之前虽然已经是一个独立模块，但它内部还是把三类事情放在一起：

- detection 文件扫描
- stopword / scoring heuristics
- facade 和最终聚合排序

这和前面已经收口过的 `room_detector` 很像：主题都相关，但文件 IO、规则逻辑、对外入口其实不是同一类职责。

# 主要目标

这次提交的目标是把 `entity_detector.rs` 继续拆成：

- scan 一层
- scoring 一层
- facade 一层

同时保持外部 `crate::entity_detector::*` surface 不变。

# 改动概览

这次新增了两个内部模块：

- `rust/src/entity_detector_scan.rs`
- `rust/src/entity_detector_score.rs`

拆分后的职责边界是：

## `entity_detector_scan`

负责：

- `scan_for_detection()`
- noise directory skip rules
- prose / readable extension 策略

也就是“检测前先去哪里找文件”。

## `entity_detector_score`

负责：

- `is_stopword()`
- `score_person()`
- `score_project()`
- stopword / verb / hint 常量

也就是“候选词出现之后，怎么算它更像人名还是项目名”。

## `entity_detector`

现在只保留：

- `DetectedEntities`
- `detect_entities()`
- `detect_entities_for_registry()`
- facade 测试锚点

也就是统一入口、候选抽取、最终排序与结果组装。

# 关键知识

## 1. 检测类模块很适合按 scan 和 score 两层切

很多启发式 detector 都有这两个天然阶段：

- 先找输入材料
- 再对候选做规则判断

把它们拆开之后，后续如果要：

- 扩大/缩小扫描范围
- 调整 stopword
- 调整 person/project scoring

就不会一直挤在同一个文件里。

## 2. facade 只保留“候选抽取 + 排序 + 结果组装”，会更容易读

这次把 scan 和 score 拆走之后，`entity_detector.rs` 剩下的就是最核心的主流程：

1. 扫描文件
2. 提取候选词
3. 计算 person/project score
4. 排序并生成结果

这种结构对于后续读代码的人更友好，因为入口文件现在更接近“流程图”。

# 补充知识

## 1. stopword 和 scoring 常量单独放出去，后面调整噪声会更集中

像：

- `STOPWORDS`
- `PERSON_VERBS`
- `PROJECT_HINTS`

这种常量，随着真实数据和误报情况变化，往往会频繁调整。

把它们和 scoring 函数放在一起，比散在 facade 文件里更容易维护。

## 2. 这种拆分能让相邻模块的内部结构更对称

前一轮 `room_detector` 拆成了：

- config
- detect

这一轮 `entity_detector` 拆成了：

- scan
- score

虽然命名不完全一样，但结构思路是一致的：把“外部入口”和“内部规则细节”拆开，让同类 heuristics 模块都更薄。

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

- 没有改变 entity detector 的外部行为
- 没有改变 people/projects 的最终排序规则
- 没有改变 registry/bootstrap 对 detector 的调用方式
- 没有继续拆 candidate regex 或最终排名逻辑
- 没有改 Python `entity_detector.py`
