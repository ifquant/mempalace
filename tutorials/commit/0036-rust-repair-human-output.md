# 0036 Rust repair 的人类可读输出

这次提交把 Rust CLI 的 `repair` 也补成了双输出模式。

## 背景

Python 的 `repair` 命令天然就是终端文本工具：

- 标题
- palace 路径
- drawer 数
- 中间进度
- 最后 repair complete

而 Rust 当前的 `repair` 其实不是“重建 collection”，只是**非破坏性诊断**。  
所以这次不能简单照抄 Python 文案，而要让输出风格像 Python、语义又忠于 Rust 当前能力。

## 这次做了什么

Rust `repair` 新增了：

- `--human`

加上后会输出人类可读诊断摘要：

- `MemPalace Repair`
- `Palace: ...`
- `SQLite: present/missing`
- `LanceDB: present/missing`
- `Drawers found: ...`
- `Schema version: ...`
- `Embedding: provider/model/dimension`
- `Vector access: ok/failed`
- 如果有问题，逐条列出 `Issues`

没有 palace 时，也会像 Python 那样直接打印：

- `No palace found at ...`

## 为什么不打印“Repair complete”

因为 Rust 这里当前没有真正执行 rebuild。

如果打印：

- `Repair complete`

就会误导用户，以为我们真的做了修复动作。  
但当前实现做的是：

- 路径检查
- schema 检查
- embedding profile 检查
- LanceDB 可访问性检查

所以更准确的文案是：

- `Repair diagnostics look healthy.`

## 这次补的回归

补了 3 条 CLI 测试：

- `repair --help` 提示 human 模式
- `repair --human` 在 no-palace 场景下输出文本
- `repair --human` 在健康 palace 上输出诊断摘要

## 一个小知识点

做 CLI 对齐时，最容易犯的错是“只模仿表面文字，不检查语义是否还成立”。

更稳的做法是：

1. 先判断 Python 输出背后的真实动作
2. 再决定 Rust 该保留哪些外观
3. 对不成立的部分换成更诚实的文案

这次 `repair --human` 就是这种处理：

- 风格像 Python
- 语义忠于 Rust 当前实现
