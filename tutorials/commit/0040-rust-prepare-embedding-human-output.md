# 0040 Rust prepare-embedding 的人类可读输出

这次提交把 Rust CLI 的 `prepare-embedding` 也补成了双输出模式。

## 背景

到这里，Rust 版的很多主命令已经都有：

- 默认 JSON
- `--human`

但 `prepare-embedding` 还只有 JSON。  
这在本地模型首次准备时不太顺手，因为用户往往只是想快速确认：

- provider 是谁
- model 是谁
- 尝试了几次
- 成没成功
- 模型文件有没有落地

## 这次做了什么

Rust `prepare-embedding` 新增了：

- `--human`

加上后会打印一个简洁的 embedding 准备摘要：

- `MemPalace Prepare Embedding`
- `Palace`
- `Provider`
- `Model`
- `Attempts`
- `Result`
- `Last err`
- `Warmup`
- `Model dir`
- `Model file`
- `Model file present`

默认不加 `--human` 时，仍然保持原来的 JSON。

## 为什么这个命令适合 human 模式

因为它和 `doctor` 很像，典型使用场景是：

- 机器第一次装模型
- 网络/镜像有问题
- 想确认 cache 是否准备好了

这时用户最关心的不是完整 JSON，而是“现在成功了没有、卡在哪儿”。  
人类可读摘要更适合快速扫一眼。

## 这次补的回归

补了 2 条 CLI 测试：

- `prepare-embedding --help` 会提示 human 模式
- `prepare-embedding --human` 会打印 embedding 准备摘要

## 一个小知识点

诊断链路里的命令通常最好成对设计：

- `doctor` 负责看状态
- `prepare-embedding` 负责做准备动作

如果两者的输出风格差太大，排查时心智会断。  
所以这次也是沿着 `doctor --human` 的风格继续往前收口。
