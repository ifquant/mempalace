# 0029 Rust 搜索结果语义继续对齐 Python

这次提交继续收紧 Rust `search` 的结果组织方式，让它更贴近 Python `search_memories()`。

## Python 版到底做了什么

Python 的 `search_memories()` 其实很克制：

- 直接保留向量库返回的结果顺序
- 不做文件级去重
- `similarity` 四舍五入到 3 位小数
- `source_file` 总是取 basename

也就是说，它不会把同一个文件的多个 chunk 折叠成一条结果。  
如果一个文件里有 2 个相关 drawer，Python 就会返回 2 条。

## Rust 这次收紧了哪些点

Rust 现在把这些语义固化到了 service 层：

- `source_file` 会规整成 basename
- `similarity` 会统一 round 到 3 位小数
- 重复文件命中不会被折叠
- 结果顺序会做稳定化处理

这里的“稳定化处理”是：

1. 先按 `similarity` 从高到低
2. 再按 `source_file`
3. 再按 `chunk_index`
4. 最后按 `id`

这样做不是为了发明新的产品语义，而是为了把“同分结果的顺序”固定下来，避免不同运行里 JSON 顺序漂动，导致测试和回归难看。

## 为什么要在 service 层做

如果把这些规则散在 CLI 和 MCP 两层，会很快出现分叉：

- CLI 自己 round 一次
- MCP 再 round 一次
- 后续别的调用点又各写一套

所以更稳的做法是：

- service 层统一后处理
- CLI 和 MCP 直接复用

这样后面无论是谁调 `App::search()`，拿到的都是同一套结果语义。

## 这次补的测试

补了 2 条 service 层单测：

- `source_file` 会规整为 basename，`similarity` 会 round 到 3 位小数
- 同一文件的多个 chunk 结果会保留，不会被去重

同时保留了原有 CLI 搜索 JSON 回归，确保外层 shape 没被打坏。

## 一个小知识点

“更像 Python”不一定意味着“逐字节复制 Python 的偶然行为”。

像结果顺序这类地方，如果底层向量库没有明确承诺 tie-break 顺序，那么：

- Python 现在看起来稳定
- 但这可能只是底层库的偶然输出

Rust 这里显式加稳定排序，属于 **把隐含行为变成显式契约**。  
这通常比盲目模仿“当前刚好这样”更适合长期维护。
