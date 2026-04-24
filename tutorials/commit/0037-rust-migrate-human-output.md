# 0037 Rust migrate 的人类可读输出

这次提交把 Rust CLI 的 `migrate` 也补成了双输出模式。

## 背景

Python 的 `migrate` 是典型的运维命令，默认就会打印：

- 标题
- palace 路径
- 数据库路径
- 版本信息
- 迁移结果

而 Rust 之前只有 JSON 摘要。

这和 `search / status / repair` 之前的问题一样：

- 自动化喜欢 JSON
- 终端里的真人更想看简洁文本

## 这次做了什么

Rust `migrate` 新增了：

- `--human`

加上后会打印人类可读摘要：

- `MemPalace Migrate`
- `Palace`
- `SQLite`
- `Before`
- `After`
- `Migration complete.` 或 `Nothing to migrate.`

## 为什么只改 human 模式

这里特意没改默认 JSON 路线。

原因是 Rust 当前的 `migrate` 语义和 Python 并不完全一样：

- Python 的 migrate 是“跨 ChromaDB 版本重建 palace”
- Rust 的 migrate 目前是“升级 SQLite schema version”

所以这次最稳的做法是：

- 默认 JSON 继续保持现有机器接口
- `--human` 只补足人类可读体验

这样既对齐使用感受，又不假装两边实现完全相同。

## no-palace 为什么直接打印文本

和 `status --human`、`repair --human` 一样，一旦用户显式选择 human 模式，最好整条命令都保持人类文本协议：

- 成功时是文本
- no-palace 时也是文本

否则同一个模式里一会儿文本、一会儿 JSON，会很割裂。

## 这次补的回归

补了 3 条 CLI 测试：

- `migrate --help` 会提示 human 模式
- `migrate --human` 在 no-palace 场景下输出文本
- `migrate --human` 在 legacy schema 上输出迁移摘要

## 一个小知识点

对齐 CLI 时，不一定要先把底层能力完全做成一样，才能先对齐外层体验。

更实用的顺序通常是：

1. 先把输出界面、错误路径、帮助文案收紧
2. 再继续把底层实现逐步靠拢

这样用户更早就能感受到“这是同一个工具家族”，而不是等到所有内部细节都重写完才开始收口。
