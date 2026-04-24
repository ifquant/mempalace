# 背景

前一轮已经把 Rust CLI 的 help 文案往 Python 版靠了一层，但“命令怎么介绍自己”只是接口的一部分。  
真正被脚本、agent 和外部调用消费的，还是命令输出本身。

这时候 Rust `search` 还有一个明显差距：

- CLI JSON 主要只有 `results`

而 Python 的程序化搜索接口 `search_memories()` 返回的是：

- `query`
- `filters`
- `results[*].source_file`
- `results[*].similarity`

这类字段对上层使用很重要，因为它们避免调用方自己重复推导上下文。

# 主要目标

这次提交的目标是让 Rust CLI 的 `search` JSON 更像 Python 的程序化接口：

1. 补 `query`
2. 补 `filters`
3. 补每条结果的 `source_file`
4. 补每条结果的 `similarity`

# 改动概览

主要改动如下：

- `rust/src/model.rs`
  - `SearchResults` 新增：
    - `query`
    - `filters`
  - 新增 `SearchFilters`
  - `SearchHit` 新增：
    - `source_file`
    - `similarity`
- `rust/src/storage/vector.rs`
  - 在读取 LanceDB 查询结果时直接补出：
    - `source_file`
    - `similarity`
  - 同时保留原始 `source_path` 和 `score`
- `rust/src/service.rs`
  - `search()` 现在返回完整的：
    - `query`
    - `filters`
    - `results`
- `rust/src/mcp.rs`
  - MCP `mempalace_search` 改为直接复用新的结构化字段，而不是再次手工推导
- `rust/tests/cli_integration.rs`
  - 新增 `cli_search_json_matches_python_style_shape`
  - 同时补强 fastembed smoke test 里的 `query/filters` 断言
- `rust/README.md`
  - 记录 `search` CLI JSON 已向 Python shape 靠拢

# 关键知识

## 1. 程序化接口最怕“调用方自己补语义”

如果 `search` 结果只给你 `results[]`，调用方通常还得自己维护：

- 原查询是什么
- 过滤条件是什么
- `source_path` 怎么转成更适合展示的文件名
- 分数怎么转成 similarity

这会造成两个问题：

- 每个调用方都重复写一遍
- 很容易出现不同地方解释不一致

所以把这些字段放回接口本身，长期成本更低。

## 2. `source_path` 和 `source_file` 同时存在是有价值的

有些场景需要完整路径：

- 调试
- 重建索引
- 深链回源

有些场景只需要展示名：

- CLI 结果
- MCP 返回
- agent 生成引用时

这就是为什么这次不是“二选一”，而是：

- 保留 `source_path`
- 额外补 `source_file`

# 补充知识

## 为什么 `similarity` 仍然保留 `score` 旁边，而不是完全替代

当前 LanceDB 返回的是 `_distance`，Rust 内部还保留了原始 `score` 字段。  
这样做有两个好处：

- 对外接口更友好：`similarity`
- 对内部调试更诚实：`score`

重写阶段保留这两层信息，通常比过早只留一种更稳。

## 为什么这次顺手让 MCP 直接复用新的 `search` 结构

因为 MCP 本来就在做 Python 风格输出适配。  
一旦 service 层已经把：

- `query`
- `filters`
- `source_file`
- `similarity`

这些字段准备好了，MCP 再手工重复推导就没有价值了。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增关键验证：

- `cli_search_json_matches_python_style_shape`

这个测试会验证：

- `query`
- `filters.wing`
- `filters.room`
- `results[0].source_file`
- `results[0].similarity`

都已经出现在 Rust CLI 的 JSON 里。

# 未覆盖项

这次没有继续做：

- `status/migrate/repair` 的更深度 shape 统一
- CLI 人类可读输出继续模仿 Python 的终端排版
- `search` 的 no-palace CLI 友好提示

所以这次提交的定位是：  
先把 Rust `search` 的结构化 JSON 提升到更像 Python 程序化接口的水平。
