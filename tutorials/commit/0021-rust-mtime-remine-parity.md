# 0021 Rust 项目重挖语义向 Python 对齐

这次继续收紧 Rust `mine` 的项目模式，把“重复挖掘时什么时候跳过、什么时候重挖”往 Python `miner.py` 靠。

## 做了什么

- SQLite `ingested_files` 新增 `source_mtime`
- schema version 从 `2` 升到 `3`
- Rust `mine` 现在会优先用 `mtime` 判断文件是否未变化
- 如果没有可用 `mtime`，或者 `mtime` 不匹配，仍然会继续用内容哈希兜底
- 新增真实回归测试，覆盖：
  - 未修改文件再次 `mine` 会跳过
  - 只改 `mtime` 也会触发重新挖掘

## 为什么这样做

Python 版项目 miner 的核心判断是：

1. 先看这个文件以前有没有被挖过
2. 如果挖过，再看 `source_mtime` 是否和当前文件一致
3. 一致就跳过，不一致就重挖

Rust 之前只看内容哈希。  
这虽然也能避免重复写入，但它和 Python 的行为不完全一样：

- Python 更偏向“文件改了时间就当作要重新过一遍”
- Rust 之前更偏向“内容没变就不动”

为了继续往 Python 对齐，这次把 `mtime` 提到了主判定路径，同时保留哈希兜底，避免没有时间戳时退化。

## 测试

跑了这些验证：

```bash
cd rust && cargo fmt --check
cd rust && cargo test
cd rust && cargo clippy --all-targets --all-features -- -D warnings
```

新增覆盖：

- `mine_skips_unchanged_files_and_remines_when_mtime_changes`

## 新手知识点

“是否需要重处理”这类逻辑，常见有两种信号：

- `mtime`：快，便宜，但可能会因为只改时间而触发重算
- 内容哈希：更稳，但必须先把内容读出来

工程里常见做法不是二选一，而是分层：

1. 先用便宜信号快速跳过大多数未变文件
2. 再用更稳的信号做兜底

这样能在性能、兼容性和正确性之间取一个比较实际的平衡。
