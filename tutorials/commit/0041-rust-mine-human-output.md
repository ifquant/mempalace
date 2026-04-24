# 0041 Rust mine 的人类可读摘要

这次提交把 Rust CLI 的 `mine` 也补成了双输出模式。

## 背景

之前 `mine` 的情况比较特别：

- 默认输出是 JSON summary
- `--progress` 会把逐文件事件打到 `stderr`

这已经比最初版本好了很多，但还少了一块：

- 最终摘要如果是给人看，还是得读 JSON

所以这次继续收口：

- 默认 JSON 保持不变
- `--human` 时，把最终摘要改成人类可读文本

## 这次做了什么

Rust `mine` 新增了：

- `--human`

加上后，最终 `stdout` 会打印类似：

- `MemPalace Mine`
- `Wing`
- `Rooms`
- `Files`
- `Palace`
- `Project`
- `Files processed`
- `Files skipped`
- `Drawers filed`
- `Rooms filed`
- `next_hint`

如果同时传：

- `--human`
- `--progress`

那么行为是：

- 逐文件进度继续走 `stderr`
- 最终摘要走人类可读 `stdout`

这和之前“默认 JSON + stderr 进度”的思路是兼容的。

## 为什么不把 progress 也改成 stdout

因为 `--progress` 目前已经形成了一个很清楚的约定：

- 过程事件 -> `stderr`
- 最终结果 -> `stdout`

这一点对脚本和人工使用都很有帮助。  
所以这次没有去破坏它，而是只把“最终结果”的表达方式变成可选 human。

## 这次补的回归

补了两条 CLI 测试：

- `mine --help` 会提示 human 模式
- `mine --human --progress` 会同时满足：
  - `stdout` 是人类可读 summary
  - `stderr` 仍然有逐文件进度

## 一个小知识点

CLI 的“过程输出”和“结果输出”最好分开设计。

一个常见且稳妥的模式是：

- 过程事件放 `stderr`
- 最终结果放 `stdout`

这样不管最终结果是 JSON 还是 human 文本，都不会和过程日志掺在一起。
