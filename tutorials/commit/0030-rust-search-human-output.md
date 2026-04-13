# 0030 Rust 搜索的人类可读输出补齐

这次提交把 Rust CLI 的 `search` 又往 Python 版收紧了一步。

## 背景

Python 里其实有两条搜索路径：

- `search_memories()`：给 MCP 和程序调用，返回字典
- `search()`：给 CLI 人直接看，打印可读文本

Rust 之前只有一条：

- 永远输出 JSON

这对自动化很好，但对人在终端里临时查东西没那么顺手。

## 这次加了什么

Rust `search` 新增了：

- `--human`

加上以后，输出风格会更接近 Python：

- 顶部 banner
- `Results for: "query"`
- 可选 `Wing:` / `Room:`
- 每条结果显示：
  - `[i] wing / room`
  - `Source: file`
  - `Match: similarity`
  - verbatim 文本内容

如果没有结果，也会像 Python 一样打印：

- `No results found for: "query"`

## 为什么不是直接把默认输出改成人类文本

因为 Rust 这边已经有很多测试、MCP、脚本依赖默认 JSON 了。

如果直接把默认输出切成文本，会破坏：

- 现有 CLI 集成测试
- 自动化脚本
- 程序化调用的稳定性

所以这里采用的是更稳的兼容策略：

- 默认仍然是 JSON
- `--human` 时输出 Python 风格文本

这属于“给人类更好的体验，但不破坏机器接口”。

## 这次补的回归

补了两条 CLI 测试：

- `search --human` 会打印 Python 风格结果块
- 空 palace / 空结果场景会打印 Python 风格的 no-results 文案

## 一个小知识点

CLI 的“默认输出”和“更友好的可选输出”最好分开。

经验上：

- 默认输出应该优先稳定、机器可读
- 可选输出可以优先人类体验

这样工具才能同时服务：

- shell 脚本
- MCP / agent
- 终端里的真人用户
