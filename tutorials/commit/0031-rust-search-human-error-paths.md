# 0031 Rust 搜索人类模式的失败路径对齐

这次提交继续收紧 Rust CLI 的 `search --human`，重点不是成功结果，而是失败路径。

## 背景

上一片已经让 Rust 支持：

- 默认 `search` 输出 JSON
- `search --human` 输出 Python 风格的可读结果

但那时还有一个不一致点：

- 成功时 `--human` 是文本
- 没 palace 时却还是 JSON

这会让命令行体验很别扭，因为同一个 `--human` 模式下，成功和失败会突然切换协议。

## 这次做了什么

现在 `search --human` 在 no-palace 场景下会输出 Python 风格文本：

- `No palace found at ...`
- `Run: mempalace init <dir> then mempalace mine <dir>`

同时，如果后续搜索执行本身报错，`--human` 也会走文本路径：

- `Search error: ...`

默认 JSON 模式则完全不变，仍然返回：

- `error`
- `hint`
- `palace_path`

## 为什么这样设计

这里本质上是两个接口：

1. 默认 CLI JSON：给脚本、测试、程序化调用
2. `--human`：给终端里的真人用户

一旦进入 `--human` 模式，最好的体验就是：

- 成功时是文本
- 失败时也是文本

否则用户会遇到这种断裂：

- 正常查东西时看人类可读输出
- 一旦出错突然蹦出 JSON

这对真人来说并不友好。

## 这次补的回归

补了一条 CLI 测试，锁住：

- `search --human` 在 no-palace 场景下输出 Python 风格提示文本

## 一个小知识点

命令行工具经常同时服务两类对象：

- 人
- 机器

一个很实用的经验是：

- 默认输出优先稳定、结构化
- 显式的 human 模式优先一致、可读

不要试图让一套输出同时完美服务两边，通常会两边都不满意。
