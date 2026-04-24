# 0061 Rust MCP 只读工具错误收口

本次提交把 Rust MCP 只读工具的失败路径继续往 Python 风格收紧了一层。

## 这次改了什么

- `mempalace_status`
- `mempalace_list_wings`
- `mempalace_list_rooms`
- `mempalace_get_taxonomy`
- `mempalace_search`

现在这些工具在 palace 已存在、但读取过程失败时，不再把错误抬成 MCP transport error。
而是继续返回工具内容里的 JSON：

```json
{
  "error": "...",
  "hint": "..."
}
```

这样调用方不需要同时处理两套失败语义：

- 一套是 `result.content[0].text`
- 另一套是顶层 `error`

未知工具和未知方法仍然保持 MCP 顶层错误，这个边界没有改。

## 为什么这样改

Python 版的思路更偏“工具内部自己消化业务失败，再返回结构化结果”。

这对 agent 调用很重要，因为：

1. 更容易做统一解析
2. 更像普通业务返回，而不是 transport 崩掉
3. 可以顺手把恢复提示 `hint` 一起带回去

## 这次补了哪些回归

- broken SQLite 下：
  - `mempalace_status`
  - `mempalace_list_wings`
  - `mempalace_list_rooms`
  - `mempalace_get_taxonomy`
  都会返回工具级 `error + hint`
- `mempalace_search` 缺少 `query` 参数时，也会返回工具级 `error + hint`

## 顺手记一个知识点

MCP 里有两层错误面：

- transport / protocol error：比如未知方法、未知工具、请求格式错
- tool/business error：工具找到了，但业务执行失败

如果业务错误也全抬到 transport 层，调用方会很快变得难写。对 agent 友好的做法通常是：

- 协议问题走顶层 error
- 业务问题走工具自己的结构化 payload
