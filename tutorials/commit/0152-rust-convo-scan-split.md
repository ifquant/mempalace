# 0152 Rust `convo_scan` 内部继续拆分

## 背景

上一轮已经把 conversation ingest 里的 `convo_exchange` 拆成了 room routing 和 chunking 两层，但 `convo_scan.rs` 仍然把三件事放在一起：

- include override 路径规范化
- force-include 命中判断
- ignore-aware walk 和 conversation 文件过滤

这些逻辑都服务于“扫描对话文件”，但变化频率和关注点并不一样，继续堆在一个文件里会让后续维护越来越难。

## 主要目标

把 `rust/src/convo_scan.rs` 再往下按 include 判定和 walk/filter 两层拆开，同时保持外部 `scan_convo_files()` 调用面不变。

## 改动概览

- 新增 `rust/src/convo_scan_include.rs`
  - 承载 include override 的规范化
  - 承载 exact/path-prefix force-include 判断
- 新增 `rust/src/convo_scan_walk.rs`
  - 承载 `scan_convo_files()`
  - 承载 conversation 文件后缀/`.meta.json`/体积过滤规则
- 精简 `rust/src/convo_scan.rs`
  - 改成薄 facade
  - 只保留 public re-export 和这组扫描逻辑的测试锚点
- 更新 `rust/README.md`
- 新增本教程文档

## 关键知识

### 1. include override 和文件过滤不是一回事

这两类逻辑看起来都在“决定文件要不要进来”，但语义不同：

- include override 是用户显式强制放行
- 文件过滤是系统默认规则，例如后缀、`.meta.json`、体积限制

把它们拆开后，后面如果要调 include 规则，不会顺手碰到 walk/filter 的默认行为。

### 2. facade 稳住的是 API，不是内部结构

这次仍然保留 `convo_scan.rs` 作为 facade，所以外部继续只认：

- `scan_convo_files()`

内部怎么拆，不需要让 `convo.rs`、`miner` 或 service 层一起跟着改 import。这种“外部稳定、内部自由重组”的做法，是重写后期持续收口时很有用的技巧。

### 3. scan 逻辑最好保留单独测试锚点

扫描链路很容易在小改动里出现细碎回归，比如：

- 路径斜杠标准化不一致
- prefix include 只匹配文件不匹配父目录
- `.meta.json` 被误当正常 transcript

所以这次把测试锚点留在 facade 层，专门锁这些边界。

## 补充知识

一个实用经验是：凡是同时涉及“用户 override”和“系统默认规则”的地方，都很适合拆层。

因为这两类逻辑最常见的 bug，就是默认规则把 override 吃掉，或者 override 绕过了不该绕过的保护。把它们放在不同模块里，阅读和排错都会更直接。

## 验证

在 `rust/` 目录顺序执行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## 未覆盖项

- 这次没有改 `python/` 侧的 conversation scanning
- 这次没有改变已有的 include override 语义，只做 Rust 内部职责拆分
- 这次没有继续拆 `convo_general` 或 `miner_convo` 更深层的扫描调用路径
