# 0023 Rust 项目扫描继续向 Python `scan_project()` 对齐

这次不是补 CLI 表面，而是继续收紧 Rust 项目扫描的底层语义，重点对齐 Python `test_miner.py` 里那些容易漏掉的 `.gitignore` 边界。

## 做了什么

- `discover_files()` 现在支持目录级 `include_ignored`
- 即使目录本身属于默认跳过目录，例如 `.pytest_cache`，只要显式 include，就会继续进入扫描
- 补了 4 条对齐 Python 的回归测试：
  - 嵌套 `.gitignore` 规则
  - 父目录仍可见时的 negation
  - 整个目录被忽略时，不会错误地重新纳入单个文件
  - `include_ignored=[".pytest_cache"]` 可以覆盖默认 skip-dir 规则

## 为什么这样做

这类扫描规则如果只测“普通目录能不能扫到文件”，很容易产生一种错觉：系统已经稳定了。  
真正麻烦的是边角：

- 根目录 `.gitignore`
- 子目录自己的 `.gitignore`
- negation 规则
- 我明明手动指定了 include，系统却还在入口就把目录砍掉

Rust 之前最后一个缺口就在这里：虽然它已经支持 include 某个被忽略的文件，但对于“把整个被跳过目录重新放进来”还不够像 Python。

## 测试

跑了这些验证：

```bash
cd rust && cargo fmt --check
cd rust && cargo test
cd rust && cargo clippy --all-targets --all-features -- -D warnings
```

新增覆盖：

- `mine_respects_nested_gitignore_and_negation_rules`
- `mine_handles_gitignore_negation_only_when_parent_dir_remains_visible`
- `mine_does_not_reinclude_file_from_ignored_directory_without_override`
- `mine_include_override_beats_skip_dirs_without_gitignore`

## 新手知识点

目录遍历里有一个常见陷阱：

如果你在“进入目录之前”就把目录过滤掉了，那么后面的 negation、force include、手动 override 很可能根本没有机会生效。

所以这类功能的关键不是只看“文件过滤条件”，还要看：

1. 目录有没有被过早剪枝
2. include override 是只对文件生效，还是对目录树也生效

这也是为什么这次修复点落在 `filter_entry()`，而不是只改文件级判断。
