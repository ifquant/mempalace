# 背景

前面几轮已经把 Rust 的结构化输出做得越来越像 Python 了，但还有一个很实际的差距：

- 当 palace 根本不存在时

Rust CLI 的行为还不够像 Python。

尤其是：

- `status`
- `search`

这两个命令在 Python 里有很稳定的用户提示语义：

- `status`：提示没有 palace，但不当成严重错误退出
- `search`：提示没有 palace，并以失败退出

而 Rust 之前更偏向：

- 自动创建空目录
- 或直接暴露内部错误

这对用户和脚本都不够友好。

# 主要目标

这次提交的目标是把 Rust CLI 在 no-palace 场景下的行为往 Python 靠一层：

1. `status` 返回 Python 风格 `error + hint`
2. `search` 返回同样的 JSON
3. `search` 保持失败退出码
4. 避免在 no-palace 场景下偷偷创建空 palace

# 改动概览

主要改动如下：

- `rust/src/main.rs`
  - 新增：
    - `palace_exists()`
    - `print_no_palace()`
  - `status` 在 palace 不存在时：
    - 输出结构化 JSON：
      - `error`
      - `hint`
      - `palace_path`
    - 退出码保持成功
  - `search` 在 palace 不存在时：
    - 输出同样的结构化 JSON
    - 退出码为失败
- `rust/tests/cli_integration.rs`
  - 新增：
    - `cli_status_reports_no_palace_with_python_style_hint`
    - `cli_search_reports_no_palace_with_python_style_hint`
- `rust/README.md`
  - 记录 `status/search` 在 no-palace 下的 Python 风格行为

# 关键知识

## 1. “没有 palace”不是普通内部错误

这类场景本质上是用户工作流状态问题，而不是程序崩溃。  
所以它不适合直接把底层错误原样抛给用户。

更好的做法是像 Python 一样明确告诉用户：

- 现在没有 palace
- 下一步该跑什么

这也是为什么这次统一收敛到：

- `error`
- `hint`

而不是暴露内部库错误。

## 2. 不同命令在 no-palace 场景下的退出语义可以不同

这次一个故意保留的差异是：

- `status`：成功退出
- `search`：失败退出

这和 Python 更接近，也符合使用直觉：

- `status` 更像诊断命令，告诉你“还没建”
- `search` 更像执行命令，没有 palace 就无法完成任务

# 补充知识

## 为什么这里先只改 CLI，不直接改 service

因为 no-palace 的“用户提示语义”首先是 CLI 接口层的问题。  
service 层更适合关注：

- 存储读取
- 检索逻辑
- 数据一致性

把用户引导提示放在 CLI 层，职责更清晰，也不影响 MCP 已经存在的 `no_palace()` 逻辑。

## 为什么输出里加 `palace_path`

因为用户经常需要马上确认：

- 你说的“没有 palace”到底是哪个路径

如果没有这个字段，很多现场排查会多绕一步。  
加上它之后，脚本和人都更容易理解当前上下文。

# 验证

已完成：

- `cd rust && cargo fmt --check`
- `cd rust && cargo test`
- `cd rust && cargo clippy --all-targets --all-features -- -D warnings`

新增关键验证：

- `cli_status_reports_no_palace_with_python_style_hint`
- `cli_search_reports_no_palace_with_python_style_hint`

验证内容包括：

- 是否输出 `error + hint + palace_path`
- `status` 是否成功退出
- `search` 是否失败退出

# 未覆盖项

这次没有继续做：

- `init/mine` 的人类友好终端排版
- 更多 CLI 失败路径的 Python 语义对齐
- service 层统一的 no-palace 错误类型

所以这次提交的定位是：  
先把 Rust CLI 在“palace 尚未存在”时的行为，收紧到更接近 Python 的用户提示语义。
