# 0034 Rust MCP 搜索错误也改成工具级 payload

这次提交继续收紧搜索失败路径，但目标换成了 MCP。

## 问题是什么

之前 Rust 的 `mempalace_search` 在 no-palace 场景已经会返回工具内容里的：

- `{"error": "No palace found", ...}`

但如果 palace 存在、只是查询过程出错，它会冒泡成 JSON-RPC transport error。  
这和 Python 的 `tool_search()` / `search_memories()` 风格不一致。

对调用方来说，这会变成两套错误处理：

- 有时读工具返回内容
- 有时还得解析顶层协议错误

## 这次做了什么

现在 `mempalace_search` 在查询失败时也会返回工具级 payload：

```json
{
  "error": "Search error: ..."
}
```

也就是说，针对这个工具：

- no palace -> 工具级错误内容
- query failure -> 工具级错误内容

这样和 Python 程序化搜索的心智更接近。

## 为什么不把所有 MCP 错误都改掉

因为这里处理的是 **业务失败**，不是协议失败。

像这些情况更适合放进工具内容：

- 没 palace
- 搜索执行失败
- 参数语义正确，但业务上没法完成

而这些情况仍然适合保留为协议错误：

- JSON-RPC method 不存在
- tool name 不存在
- 请求结构本身坏掉

所以这次只收紧 `mempalace_search` 的业务失败面，不一口气改整个 MCP 框架。

## 这次怎么测

测试方法和 CLI 那边一样，还是利用一个稳定、真实的失败入口：

1. `init`
2. 手工改坏 SQLite 里的 `embedding_provider`
3. 调 `mempalace_search`

断言点是：

- 顶层响应没有 JSON-RPC `error`
- 工具内容文本里有 `{"error":"Search error: ..."}`

## 一个小知识点

在 RPC 系统里，最好尽量区分两类错误：

- **transport / protocol error**
- **tool / business error**

如果把业务失败也抬成 transport error，调用方就很容易被迫写很多不必要的分支。  
把业务失败收回工具 payload，通常更适合 agent 和自动化脚本消费。
