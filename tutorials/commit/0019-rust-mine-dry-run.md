# 0019 Rust `mine --dry-run` 预演能力

本次提交给 Rust 版 `mempalace mine` 加了一个和 Python 方向更接近的能力：`--dry-run`。

## 做了什么

- CLI 新增 `mine --dry-run`
- `MineSummary` 新增这些稳定字段：
  - `dry_run`
  - `respect_gitignore`
  - `include_ignored`
- `service::App::mine_project()` 改成支持预演模式
- 预演模式会完整执行：
  - 文件发现
  - 文本读取
  - room 路由
  - chunk 切分
  - drawer 数量统计
- 但不会执行：
  - embedding 生成
  - LanceDB 写入
  - SQLite drawer / ingest 记录写入

## 为什么这样做

Rust 版要继续向 Python 版对齐，`mine` 不能只有“直接写入”这一种模式。预演模式有两个直接价值：

- 调试项目扫描和 taxonomy 路由时更快
- 后面继续补 Python 版的 `mine` 参数时，可以复用同一条业务路径，而不是维护一套平行假实现

这里刻意没有做成第二套扫描逻辑，而是在同一个 `mine_project()` 中只切掉最终持久化步骤。这样能保证 dry-run 和真实写入使用相同的发现、分块、路由规则。

## 测试

跑了这些真实验证：

```bash
cd rust && cargo fmt --check
cd rust && cargo test
cd rust && cargo clippy --all-targets --all-features -- -D warnings
```

新增覆盖：

- `mine_dry_run_reports_work_without_writing_drawers`
- `cli_mine_dry_run_reports_preview_without_writing_drawers`

## 新手知识点

Rust 里这种“预演模式”最稳的做法，通常不是复制一份流程，而是：

1. 先让扫描、解析、路由走同一条真实链路
2. 只在最终 side effect 处做条件分叉

这样测试覆盖率更有意义，也更不容易出现“dry-run 说能挖，真实写入却失败”的语义漂移。

另一个点是：如果 dry-run 直接跳过 embedding 计算，测试会更快，也能避免为了预演而强行拉起昂贵的本地模型。
