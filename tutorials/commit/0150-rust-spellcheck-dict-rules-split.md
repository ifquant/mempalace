# 背景

`rust/src/spellcheck.rs` 之前把几类不同职责都放在一个文件里：

- transcript / user-text spellcheck 入口
- known names 加载
- common typo map
- system dictionary 和 edit distance
- token skip regex 规则

这些逻辑都属于 spellcheck，但变化频率并不一样：

- 入口层更稳定
- 字典和候选排名会随着修词策略调整
- skip 规则会随着误判案例变化

继续全堆在一个文件里，会让 spellcheck 这条线越改越难读。

# 主要目标

这次提交的目标是把 `spellcheck.rs` 继续拆成：

- dict 一层
- rules 一层
- facade 一层

同时保持外部 spellcheck API 不变。

# 改动概览

这次新增了两个内部模块：

- `rust/src/spellcheck_dict.rs`
- `rust/src/spellcheck_rules.rs`

拆分后的职责边界是：

## `spellcheck_dict`

负责：

- `COMMON_TYPOS`
- `common_typo_map()`
- `system_words()`
- `best_dictionary_candidate()`
- `edit_distance()`
- system dictionary load/index

也就是“从哪里找候选词”和“候选词怎么排名”。

## `spellcheck_rules`

负责：

- `should_skip()`
- token regex
- URL / code / camelCase / technical token 等 skip 规则

也就是“什么 token 根本不应该尝试拼写修正”。

## `spellcheck`

现在只保留：

- `spellcheck_user_text()`
- `spellcheck_transcript()`
- `known_names_for_path()`
- transcript line 入口
- 顶层测试锚点

也就是 spellcheck 的统一外部入口。

# 关键知识

## 1. 拼写修正最容易混在一起的其实是“候选生成”和“跳过规则”

做 spellcheck 时，很容易把所有逻辑都写进一个 `fix_token()`。

但实际上至少有两类问题：

- 这个 token 要不要尝试修
- 如果要修，候选词从哪里来

把这两类逻辑拆开之后，排查误判会容易很多。

## 2. facade 保留入口，能避免 normalize 链路被迫跟着改 import

这次 `normalize` 和 `convo` 上层并不需要知道：

- dict 在哪个文件
- rules 在哪个文件

它们继续只认 `spellcheck_user_text()` / `spellcheck_transcript()`。

这让内部收口可以持续进行，而不会制造无意义的上层迁移。

# 补充知识

## 1. 技术 token 的 skip 规则通常比“修得更聪明”更重要

在 agent/chat transcript 里，误伤：

- CamelCase
- URLs
- paths
- code-ish tokens
- known entity names

通常比少修一个 typo 更糟。

所以把 skip 规则独立出来，本质上是在把“别修错”当成一等目标。

## 2. system dictionary 和 edit distance 单独集中，更方便以后替换词典策略

现在 `spellcheck_dict` 把系统词典读取、索引和 edit distance 放在一起。

这样后面如果要：

- 改词典来源
- 调整最大编辑距离
- 换候选排序策略

就不用去碰 transcript 或 regex guard 那一层。

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

- 没有改变 spellcheck 的外部行为
- 没有改变 transcript 只修 user turn 的规则
- 没有改 normalize/convo 对 spellcheck 的接线方式
- 没有替换系统词典来源
- 没有改 Python `spellcheck.py`
