# 背景

前两轮已经把 Rust 的 project CLI 继续拆成：

- bootstrap family
- mining family

还剩下的 `rust/src/project_cli_transcript.rs` 虽然不大，但仍然同时装着两条不同命令线：

- `split`
- `normalize`

并且还混着：

- handler
- normalize 的 human/json 渲染
- shared JSON helper

既然 project CLI 其它 family 都已经收到了更细的粒度，这一块继续留在一个文件里就不一致了。

# 主要目标

把 Rust project transcript CLI 也收成同样的粒度，同时保持外部 surface 不变：

- `handle_split()` 继续从 `project_cli_transcript` 暴露
- `handle_normalize()` 继续从 `project_cli_transcript` 暴露
- `project_cli.rs` 和顶层 CLI dispatch 不需要改调用方式
- 用户可见的 split/normalize 行为和输出不变化

# 改动概览

这次新增了三个内部文件：

- `rust/src/project_cli_transcript_split.rs`
- `rust/src/project_cli_transcript_normalize.rs`
- `rust/src/project_cli_transcript_support.rs`

并把 `rust/src/project_cli_transcript.rs` 收成一个薄 facade。

## 1. `project_cli_transcript_split`

这里现在承接：

- `handle_split()`

`split` 这条命令本身很薄，只需要把 transcript mega-file split flow 包一下，所以单独一个小文件就够了。

## 2. `project_cli_transcript_normalize`

这里现在承接：

- `handle_normalize()`
- normalize 的 human 预览输出
- normalize 的 JSON error 输出

`normalize` 明显比 `split` 更重一些，因为它既要读原文件，也要做 unsupported transcript 的错误分流和 preview 渲染，所以单独放出去更清楚。

## 3. `project_cli_transcript_support`

这里现在承接 transcript family 共享 helper：

- `print_transcript_json()`

虽然这层很薄，但这样可以让 transcript family 保持和 bootstrap/mining 一样的结构。

## 4. `project_cli_transcript`

这个文件现在只保留 re-export：

- `handle_split()`
- `handle_normalize()`

它的职责变成“transcript command family 的薄入口”，而不再承载具体实现。

# 关键知识

## 1. 小模块也值得做结构对齐

这次拆分的收益不只是“减少行数”，更重要的是把 project CLI 三个 family 的形状统一起来：

- bootstrap
- mining
- transcript

当结构统一后，后面继续维护时就不用每次重新猜“这个 family 是不是还在用旧写法”。

## 2. `normalize` 的 renderer 应该跟命令线走

`normalize` 里真正容易继续膨胀的部分，不是 `normalize_conversation_file()` 的调用，而是：

- unsupported transcript 的错误分流
- human preview
- JSON summary shape

这些本来就是 normalize command contract 的一部分，所以这次和 `handle_normalize()` 一起留在 `project_cli_transcript_normalize` 更稳。

# 补充知识

## 为什么 `split` 没有再造更多 helper

`split` 这条线本身已经很薄，只有：

- 参数透传
- `split_directory()` 调用
- JSON 输出

这里如果继续为了“对称”再造更多 helper，反而会制造不必要的抽象。保留一个薄 handler 就够了。

## 结构一致性本身也是维护收益

连续几轮 CLI 收口后，代码里形成了一个稳定模式：

- family facade
- family support
- per-command implementation

这种一致性会直接降低下一次继续收口时的认知成本，因为你已经知道下一刀该往哪里下。

# 验证

在 `rust/` 下运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

这些检查通过后，可以确认：

- transcript CLI split 没破坏现有命令 surface
- 新的 facade + support 结构没有打断编译
- 现有 CLI / service / MCP 回归仍然保持绿色

# 未覆盖项

这次没有继续改：

- `normalize.rs`
- `split.rs`
- `project_cli_support.rs`
- `project_cli.rs`

因为目标只是把 project transcript CLI 再按命令族切开，而不是继续扩散到 transcript library 模块或更外层 dispatch。
