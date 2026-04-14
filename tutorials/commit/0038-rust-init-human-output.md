# 0038 Rust init 的人类可读输出

这次提交把 Rust CLI 的 `init` 也补成了双输出模式。

## 背景

到这一步，Rust 版很多主命令已经都有两条输出路线：

- 默认 JSON
- `--human`

但 `init` 还是只会输出 JSON。  
这会让用户在第一次使用工具时体验不一致：

- `search/status/repair/migrate` 可以选人类可读
- `init` 却只能看 JSON

## 这次做了什么

Rust `init` 新增了：

- `--human`

加上后会打印一个简洁的初始化摘要：

- `MemPalace Init`
- `Palace`
- `SQLite`
- `LanceDB`
- `Schema`
- `Palace initialized.`

默认不加 `--human` 时，仍然保持原来的 JSON。

## 为什么 `init --human` 比较简单

因为 Rust 当前的 `init` 语义本来就比 Python `cmd_init` 更窄。

Python `cmd_init` 还会做：

- 扫描实体
- 交互确认
- 房间检测

而 Rust 当前 `init` 做的是更基础的事情：

- 创建 palace 目录
- 初始化 SQLite schema
- 准备 LanceDB 表

所以这里的人类可读输出也应该忠于当前实现，只总结“初始化完成了什么”，而不是假装已经对齐到 Python 那整条流程。

## 这次补的回归

补了 2 条 CLI 测试：

- `init --help` 会提示 human 模式
- `init --human` 会打印人类可读初始化摘要

## 一个小知识点

对齐 CLI 时，可以先把“输出界面”统一，再继续把“底层行为”慢慢补齐。

这样做的好处是：

- 用户更早能建立熟悉感
- 自动化接口仍然稳定
- 不需要等所有内部实现都对齐后，才开始收口体验层
