# 0062 Rust MCP 补齐 duplicate 和 AAAK 工具

本次提交继续把 Rust 的只读 MCP 面往 Python 版收紧。

## 新增了什么

新增两个 Rust MCP 只读工具：

- `mempalace_check_duplicate`
- `mempalace_get_aaak_spec`

### `mempalace_check_duplicate`

输入：

- `content`
- `threshold`，默认 `0.9`

输出：

- `is_duplicate`
- `matches`

每条 match 现在会返回：

- `id`
- `wing`
- `room`
- `similarity`
- `content`

其中 `content` 会像 Python 版一样做截断预览，超过 200 字符就补 `...`。

### `mempalace_get_aaak_spec`

这个工具很直接，只返回：

```json
{
  "aaak_spec": "..."
}
```

这样 agent 不一定非得先调 `mempalace_status` 才能拿到 AAAK 说明。

## 为什么这两个值得先补

因为它们都属于：

1. Python 已经稳定存在
2. Rust 底层已经有足够数据
3. 实现成本低、兼容收益高

尤其 `check_duplicate`，Rust 这边其实早就有：

- 向量搜索
- drawer `id`
- `similarity`

所以只需要把结果重新组织成 Python MCP 期望的 shape。

## 这次也顺手补了什么

- `tools/list` 现在会把这两个工具一起暴露出来
- `threshold` 支持字符串和数字两种 MCP transport 输入
- `content` 缺失时，`mempalace_check_duplicate` 也会返回工具级 `error + hint`

## 顺手记一个知识点

当后端已经有“搜索结果”模型时，很多看起来像“新功能”的 MCP 工具，其实只是：

- 复用旧查询
- 重排结果 shape
- 加一点阈值/截断逻辑

这类兼容工作通常比“重新发明一个 service”便宜得多，也更稳。
