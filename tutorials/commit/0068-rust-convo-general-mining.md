# 背景

这次提交的目标，不是再补一层 CLI 壳子，而是把 Python 版里 `convo_miner.py + general_extractor.py` 的主链路真正迁到 Rust。  
在这之前，Rust 的 `mine --mode convos` 和 `--extract general` 只是在命令行边界被接受，实际会返回“暂未实现”。这意味着：

- Rust 还不能真实挖掘聊天记录
- `exchange` / `general` 两条高级语义链路都没落盘
- `--dry-run`、`--progress`、`--human` 在 convos 场景下也没有真实意义

这次要把这条高级能力补成可运行、可测试、可继续扩展的 Rust 主线。

# 主要目标

1. 让 `mempalace-rs mine --mode convos` 变成真实实现，而不是占位错误。
2. 支持两条提取语义：
   - `--extract exchange`
   - `--extract general`
3. 让 convos 继续复用现有 `SQLite + LanceDB`，而不是新开一条旁路存储。
4. 让 convos 模式下的 `--dry-run`、`--progress`、`--human` 也有真实输出。

# 改动概览

这次改动主要分成 5 块：

1. 新增 `rust/src/convo.rs`
   - 承接聊天文件扫描、格式标准化、exchange chunking、general extractor
   - 支持 `.txt/.md/.json/.jsonl`
   - 跳过 `.meta.json`、symlink、超大文件、损坏/不支持的聊天导出

2. 扩展 drawer 元数据
   - `DrawerInput` 新增：
     - `ingest_mode`
     - `extract_mode`
   - SQLite schema 升到 `v6`
   - `drawers` 表新增：
     - `ingest_mode`
     - `extract_mode`
   - LanceDB `drawers` 表也补了同名 metadata 列

3. service 层加入 convos 主链路
   - `mine_project_with_progress()` 现在变成统一入口
   - `mode=projects` 继续走旧 project miner
   - `mode=convos` 分流到新的 conversation miner
   - conversation re-mine 复用现有 source-based replace 语义

4. CLI 收口
   - 去掉了原来 `mode != projects` 的硬拒绝
   - convos 模式现在可真实执行
   - `--progress` 新增 general 模式的 memory-type 预览
   - `--human` 摘要也会显示 `mode/extract`

5. 回归测试
   - 新增 convos/exchange/general 的 CLI smoke tests
   - 新增 service 级测试，覆盖：
     - 跳过 `.meta.json` / symlink / 超大文件
     - broken JSON 跳过但不整批失败
     - exchange re-mine 替换旧 chunks
     - general extractor 的 5 类 memory type
     - convo room bucket 对齐

# 关键知识

## 1. 为什么要把聊天标准化独立到 `convo.rs`

如果把聊天格式解析继续塞进 `service.rs`，很快就会出现两个问题：

- service 层同时负责“业务编排”和“文件格式理解”
- 后面要补更多聊天导出格式时，service 会越来越难维护

这次把它拆成单独模块后，职责更清楚：

- `convo.rs`：理解聊天文件
- `service.rs`：决定什么时候扫描、什么时候替换、什么时候写库

## 2. `general` 提取为什么先保持纯规则

这次没有引入 LLM，也没有引入远程依赖，而是沿用 Python 的思路：

- marker scoring
- prose-only filtering
- sentiment / resolution disambiguation
- min confidence threshold

这样做的好处是：

- 本地优先设计不被破坏
- 行为更稳定，可测试性更强
- 以后如果要叠加模型，也还有一个明确的基线版本

## 3. 为什么 convos 也要写 `ingest_mode/extract_mode`

如果不把这两个字段作为一等元数据落盘，很多后续能力都会变得模糊：

- 这条 drawer 到底来自 project miner 还是 convo miner？
- convo miner 是 `exchange` 还是 `general` 提取出来的？

现在写进 SQLite 和 LanceDB 之后，后面的：

- 搜索调试
- MCP 扩展
- 审计分析
- 导出/迁移

都会更容易做。

# 补充知识

1. Rust 里像 `ignore::WalkBuilder::filter_entry()` 这种闭包，经常要求 `'static`。  
   如果你把外部借用的 `&[&str]` 直接 capture 进去，就会碰到“borrowed data escapes outside of function”。这次的修法是先把 `skip_dirs` 拷成拥有所有权的 `HashSet<String>`，再 move 进闭包。

2. 规则提取器的阈值很容易“看起来合理，但实际太苛刻”。  
   这次 `general_extractor` 第一次实现时，单个强 marker 的段落会被 `0.3` 门槛挡掉，导致测试里“resolved problem -> milestone”直接空结果。最后不是重写整套规则，而是把 confidence 的归一化从 `/5.0` 调成了更贴近 Python 实际感觉的 `/3.0`。

# 验证

执行了这些真实验证：

```bash
cd /Users/dev/workspace2/agents_research/mempalace/rust
cargo check
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

这轮新增并跑过的关键回归包括：

- `cli_mine_convos_exchange_smoke`
- `cli_mine_convos_general_smoke`
- `cli_mine_convos_dry_run_reports_room_counts`
- `cli_mine_convos_human_prints_python_style_summary`
- `service_mine_convos_skips_meta_json_symlink_and_large_files`
- `service_mine_convos_exchange_replaces_existing_source_chunks`
- `service_general_extractor_classifies_decision_preference_milestone_problem_emotional`
- `service_convo_room_detection_matches_python_keyword_buckets`

# 未覆盖项

- 这次还没有做 AAAK generation / `compress` / `wake-up`
- 还没有做 onboarding / entity registry / entity bootstrap
- 还没有把 conversation mining 暴露成 MCP mine 工具
- JSON 聊天标准化目前只覆盖 Python 当前主路径和这轮测试覆盖的几类导出，不追求一次覆盖全部边缘格式
