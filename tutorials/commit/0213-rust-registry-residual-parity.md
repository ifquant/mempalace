# 背景

这次工作发生在 `rust/` 子树，目标是补齐 Rust `EntityRegistry` 相对 Python 实现还剩下的三处残余差异。

Task 3 聚焦的是 registry 最基础、但又很容易在空项目和首次初始化时踩到的行为：

- 当 `entity_registry.json` 还不存在时，Rust 之前默认返回 `mode = "work"`，但 Python 默认是 `personal`。
- onboarding seed 之前会把空名字直接写进 `people`，导致 registry 里出现空 key。
- lookup 之前只看 `people` 和 `projects`，即使 `wiki_cache` 里已经有用户确认过的条目，也会把结果返回成 `unknown`。

这些问题单看都不大，但组合起来会让 Rust 端在“刚建目录、刚 onboarding、刚确认研究结果”这条路径上持续偏离 Python。

# 主要目标

- 让空 registry 的默认模式回到 Python 同款的 `personal`。
- 让 seed 阶段跳过空白名字，避免把无效 onboarding 数据落盘。
- 让 lookup 在 `people` / `projects` 未命中时，优先返回已确认的 `wiki_cache` 条目。
- 用聚焦测试先复现失败，再验证修复后的行为。

# 改动概览

- 在 `rust/tests/parity_registry_kg_ops.rs` 新增三个聚焦测试：
  - `registry_load_defaults_to_personal_mode_like_python`
  - `registry_seed_skips_blank_names_like_python`
  - `registry_lookup_returns_confirmed_wiki_cache_entry_before_unknown`
- 在 `rust/src/registry_io.rs` 修正：
  - `EntityRegistry::load()` 缺文件时改为 `Self::empty("personal")`
  - `seed()` 先 `trim()` 名字并跳过空字符串
  - alias / canonical 写入统一基于清洗后的名字
- 在 `rust/src/registry_lookup.rs` 增加 confirmed `wiki_cache` 分支：
  - 只有 `confirmed == true` 才参与 lookup
  - 类型优先用 `confirmed_type`，没有时退回 `inferred_type`
  - 采用和 Python 一致的大小写不敏感匹配
- 新增本教程文件 `tutorials/commit/0213-rust-registry-residual-parity.md`

# 关键知识

- `load()` 的默认值不是随便选一个 mode 就行。Rust runtime 很多 registry 入口会直接在“文件还不存在”时调用 `EntityRegistry::load()`，所以这里的默认 mode 实际上就是首次使用体验的一部分。
- seed 阶段清洗数据比后面补救更划算。空名字一旦写进 `people`，后面 summary、lookup、query、导出都可能被污染。
- confirmed `wiki_cache` 不是“research 历史记录”那么简单，它在 Python 里已经参与正式 lookup 语义，所以 Rust 也必须把它当成分类来源的一部分。

# 补充知识

- 这类 parity 修复适合先钉非常小的失败测试，因为你能直接看到“现在到底差在哪一条返回值上”，不会被更大的集成流程掩盖。
- 做 registry 这类结构化状态修复时，`trim()` 这种输入清洗最好尽量靠近写入口实现，而不是依赖上游所有调用者都先做对。

# 验证

在 `rust/` 目录运行：

```bash
cargo test --test parity_registry_kg_ops registry_load_defaults_to_personal_mode_like_python --quiet
cargo test --test parity_registry_kg_ops registry_seed_skips_blank_names_like_python --quiet
cargo test --test parity_registry_kg_ops registry_lookup_returns_confirmed_wiki_cache_entry_before_unknown --quiet
cargo test --test parity_registry_kg_ops parity_registry_lookup_uses_context_to_disambiguate_name --quiet
cargo test --test service_integration registry_summary_lookup_and_learn_work --quiet
```

结果：

- 三个新增 registry parity 测试先失败后通过
- 既有上下文歧义 lookup 测试通过
- 既有 service 级 registry round-trip 测试通过

# 未覆盖项

- 这次没有修改 `python/` 子树，也没有碰 `python/uv.lock`。
- 这次没有修改 `docs/superpowers/`、`docs/rust-python-deep-gap-audit.md`、`docs/rust-python-deep-gap-list.md`。
- 这次没有改 `registry_types`，也没有扩展 CLI / MCP 输出格式。
