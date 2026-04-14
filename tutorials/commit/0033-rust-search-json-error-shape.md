# 0033 Rust 搜索 JSON 错误输出对齐

这次提交继续补 `search` 的失败路径，但目标不是 `--human`，而是默认 JSON 模式。

## 问题是什么

之前 Rust 的 `search` 在两类失败下表现不一致：

- 没 palace：已经会打印结构化 JSON
- 搜索执行过程中失败：会直接退回 Rust 错误路径

这对脚本和测试很不友好，因为调用方会碰到两种协议：

- 有时拿到 JSON
- 有时拿不到结构化输出，只能靠 stderr 文本猜

## Python 版是怎么做的

Python `search_memories()` 在查询失败时会返回：

- `{"error": "Search error: ..."}`

所以 Rust 默认 JSON 模式这次也往这个方向收紧了。

## 这次做了什么

现在默认 `search` JSON 模式下：

- 如果搜索执行失败
- CLI 会输出结构化 JSON：

```json
{
  "error": "Search error: ..."
}
```

并保持失败退出码。

这意味着：

- 人类模式 `--human`：文本错误
- 默认 JSON 模式：结构化错误

两边各自自洽了。

## 这次怎么测

和上一片一样，测试没有去伪造底层向量库崩溃，而是手工制造一个真实的 embedding profile mismatch：

1. 先 `init`
2. 再改坏 SQLite 的 `embedding_provider`
3. 然后直接跑默认 `search`

断言点是：

- 退出码失败
- stdout 里有 `"error"`
- 文案里有 `Search error:`

## 一个小知识点

CLI 的错误输出最好也分“人类协议”和“机器协议”。

经验上：

- human 模式：优先短文本、可读
- 默认 JSON 模式：优先稳定字段、可解析

只要这两条线清楚，后续扩功能时就不容易再打架。
