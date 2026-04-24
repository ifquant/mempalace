# 0039 Rust doctor 的人类可读输出

这次提交把 Rust CLI 的 `doctor` 也补成了双输出模式。

## 背景

现在 Rust 版很多常用命令都已经有：

- 默认 JSON
- `--human`

但 `doctor` 还只有 JSON。  
而 `doctor` 本身就是给人排查 embedding/runtime 问题的命令，所以它特别适合补人类可读输出。

## 这次做了什么

Rust `doctor` 新增了：

- `--human`

加上后会打印 embedding 诊断摘要：

- `MemPalace Doctor`
- `Palace`
- `SQLite`
- `LanceDB`
- `Provider`
- `Model`
- `Dimension`
- `Cache dir`
- `Model dir`
- `Model file`
- `Cache hit`
- `Model file present`
- `ORT dylib`
- `HF endpoint`
- `Warmup`

默认不加 `--human` 时，仍然输出原来的 JSON。

## 为什么这个命令特别值得做人类模式

因为 `doctor` 的目标用户通常不是脚本，而是正在排查环境问题的人。

这类场景里，用户最关心的是：

- 现在到底用了什么 provider
- 模型文件有没有落地
- ORT 动态库找到了没有
- warmup 成功还是失败

这些信息如果直接读 JSON 当然也行，但文本摘要会更快扫一眼得出结论。

## 这次补的回归

补了 2 条 CLI 测试：

- `doctor --help` 会提示 human 模式
- `doctor --human` 会打印 embedding 诊断摘要

## 一个小知识点

“诊断命令”和“数据命令”在输出设计上通常不一样。

像 `doctor` 这种命令：

- 默认 JSON 适合脚本/CI
- human 模式适合现场排查

而且 human 模式不一定非要追求和 Python 完全逐字一致，关键是把排查者最需要的信息用稳定顺序列出来。
