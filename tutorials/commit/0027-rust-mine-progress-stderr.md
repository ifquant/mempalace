# 0027 Rust `mine --progress` 逐文件进度输出

这次把 Python 项目 miner 里比较有用的一层“逐文件过程输出”搬到了 Rust CLI，但做法刻意更稳。

## 做了什么

- `mempalace-rs mine` 新增 `--progress`
- 打开后会把逐文件事件打印到 `stderr`
- 最终 summary 仍然保持 JSON，继续打印到 `stdout`

支持两种事件：

- live 模式：

```text
[   1/12] auth.txt                                           +3
```

- dry-run 模式：

```text
[DRY RUN] auth.txt -> room:auth (3 drawers)
```

## 为什么这样做

Python 版确实有价值很高的逐文件反馈：

- 可以看 room 路由有没有跑偏
- 可以看大目录卡在哪个文件
- 可以确认 dry-run 的预测结果

但 Rust 版如果直接把默认输出改成人类文本，会立刻破坏：

- JSON 稳定性
- 自动化脚本
- CLI 测试

所以这里选的是双通道方案：

- `stdout`: 最终 JSON summary
- `stderr`: 可选的人类进度流

## 测试

通过的最低成本验证：

```bash
cd rust && cargo fmt --check
cd rust && cargo check
```

另外补了 CLI 回归：

- `cli_mine_progress_prints_to_stderr_while_stdout_stays_json`
- `cli_mine_dry_run_progress_prints_python_style_preview_to_stderr`

## 新手知识点

CLI 设计里一个很好用的原则是：

- **机器输出走 stdout**
- **人类过程信息走 stderr**

这样可以同时满足两类使用方式：

1. 人手直接运行，能看到过程
2. 脚本管道读取，仍能拿到干净 JSON

这比“人类模式 / 机器模式”分成两套完全不同协议更稳，也更容易长期维护。
