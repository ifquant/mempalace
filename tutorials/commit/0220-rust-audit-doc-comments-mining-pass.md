# 背景

这份教程是一次补档 closeout，不对应新的行为改动，而是回补 commit `24415793e7568a7d746b522108ae927799c17ecc` 的提交说明。那次提交已经给 Rust 的 mining、conversation、normalize、entity/room detector、spellcheck 这些链路补了大量审计注释，但漏掉了仓库规则要求的配套教程文件。

这次 follow-up 的目标很窄：

- 不 amend 历史
- 只补文档闭环
- 让 reviewer 能把 `24415793` 这一轮 comment pass 放回整个 `0218 -> 0219 -> 0220` 审计注释序列里理解

# 主要目标

把缺失的 mining / transcript 注释切片讲清楚，尤其是 reviewer 在读相关模块时最容易问的几类问题：

- project mining 和 conversation mining 的总入口分别在哪里
- transcript normalize 为什么强调 “best effort, never drop content”
- exchange chunking、general-memory extraction、spellcheck、entity/room detector 为什么拆成多个薄模块
- 哪些注释是在解释产品边界，哪些注释是在解释容易误读的 fallback 语义

# 改动概览

- 回补了本教程文件，对应已落地的 commit `24415793e7568a7d746b522108ae927799c17ecc`
- 补充说明了那次 comment pass 涵盖的 Rust 文件族：
  - `miner.rs`、`miner_project.rs`、`miner_convo.rs`、`miner_support.rs`
  - `convo.rs`、`convo_exchange*.rs`、`convo_general*.rs`、`convo_scan*.rs`
  - `normalize.rs`、`normalize_json*.rs`、`normalize_transcript.rs`
  - `spellcheck.rs`、`spellcheck_dict.rs`、`spellcheck_rules.rs`
  - `entity_detector*.rs`
  - `room_detector*.rs`
  - `split.rs`
- 在 `rust/README.md` 里补了一个很短的 audit-reader 入口说明
  - 让读者先从 `service` 和 `service_*` 家族看整体路由
  - 再顺着 `service_project -> miner/normalize` 进入 transcript/project ingest 链路

# 关键知识

这一个切片的核心不是“增加更多注释”，而是把 transcript ingest 相关的系统边界讲清楚。

第一层是 orchestration：

- `service_project` 是 CLI / MCP / integration tests 进入项目初始化、项目 mining、conversation ingest、normalize/split 等流程的高层入口
- `miner` 只是一个薄 facade，用来把 source-file mining 和 conversation mining 两条路径拆开
- `miner_project` 关注项目文件扫描、chunking、room routing、drawer 组装
- `miner_convo` 关注 conversation 文件发现、normalize 后的 transcript extraction、general-memory/exchange 路由

第二层是 transcript normalization：

- `normalize` 负责统一入口
- `normalize_json_exports` / `normalize_json_jsonl` 分别处理 JSON 导出和 JSONL 会话导出
- `normalize_transcript` 负责 quote-line transcript 语义
- `spellcheck*` 只在需要时辅助 transcript 文本修正

这层最关键的产品语义是：Rust 这里保持 Python 兼容的 raw fallback，不因为 JSON 不认识、格式有瑕疵、或者解析失败就直接把内容丢掉。也就是注释里明确写的那种 “best effort, never drop content” 思路。

第三层是 heuristics：

- `convo_exchange*` 解释“对话轮次”怎么切块
- `convo_general*` 解释“长文本记忆”怎么抽取 decision / preference / problem 等片段
- `entity_detector*` / `room_detector*` 解释 bootstrap / mining 时的人物、项目、房间推断边界

这些模块被拆薄以后，注释的作用就不是替代码逐行翻译，而是告诉 reviewer：每一组启发式到底承担哪一段职责，失败时 fallback 到哪里，以及为什么不把所有规则重新塞回一个大文件里。

# 补充知识

1. 回补教程时，最有价值的不是复制 `git show` 输出，而是把那次提交的阅读顺序补出来。这里推荐的顺序是：`service_project` -> `miner` -> `normalize` -> `convo_*` / `spellcheck*` -> `entity_detector*` / `room_detector*`。这样能先看 orchestration，再看 transcript fallback，最后看 heuristics。

2. 对 transcript 系统来说，注释经常是在保护“不要错误丢内容”的产品约束，而不是只解释代码风格。像 malformed JSON fallback 到 raw text 这种注释，真正目的是让 reviewer 知道这里优先保留用户内容，而不是追求输入格式的纯净性。

# 验证

```bash
cd /Users/dev/workspace2/agents_research/mempalace && git diff --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo fmt --check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo check
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo test
cd /Users/dev/workspace2/agents_research/mempalace/rust && cargo clippy --all-targets --all-features -- -D warnings
```

# 未覆盖项

- 这次 closeout 不修改任何 Rust 源码逻辑，只补文档闭环
- 没有触碰 `python/uv.lock`
- 没有修改 `docs/superpowers/` 下的计划或执行文件
- 没有回写或改写历史提交；缺失教程通过 follow-up doc-only commit 补齐
