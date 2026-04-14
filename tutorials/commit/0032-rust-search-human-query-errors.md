# 0032 Rust 搜索人类模式的查询错误路径

这次提交继续补齐 `search --human` 的最后一块失败路径：

- palace 存在
- 但搜索执行本身失败

## 为什么这块还要单独补

上一片已经处理了：

- `search --human` 成功时输出文本
- `search --human` 在 no-palace 场景下也输出文本

但如果 palace 存在、真正执行搜索时才报错，CLI 仍然可能退回到默认错误路径。  
这会让 `--human` 模式再次出现“不一致协议”的问题。

## 这次做了什么

现在只要是 `search --human`：

- 成功时：打印 Python 风格结果文本
- 没 palace 时：打印 Python 风格提示文本
- 查询执行失败时：打印 `Search error: ...`

也就是说，`--human` 模式下的人类文本协议已经闭环了。

## 这次怎么测

这里没有去伪造向量库底层崩溃，而是用了更低成本、但真实会发生的错误：

- 先 `init`
- 再手工把 SQLite `meta.embedding_provider` 改坏
- 然后执行 `search --human`

这样会触发真实的 embedding profile mismatch，CLI 会走查询错误路径。

## 为什么这个测试有价值

因为它不是单纯 mock 一个异常，而是覆盖了真实用户可能遇到的问题：

- palace 是旧的
- 环境变量切了 provider
- 元数据和当前运行配置不匹配

这种错误在迁移期特别常见。

## 一个小知识点

好的 CLI 错误测试，不一定非要打最深的底层异常。

更实用的做法通常是：

- 找一个便宜、稳定、真实的失败入口
- 用它锁住用户可见行为

这里的 embedding profile mismatch 就属于这种“稳定且真实”的测试钩子。
