# 0035 Rust 状态命令的人类可读输出

这次提交把 Rust CLI 的 `status` 也补成了双输出模式。

## 背景

Python 的 `mempalace status` 默认就是人类可读文本：

- 顶部标题
- 总 drawer 数
- 每个 wing
- 每个 wing 下面的 room 和数量

而 Rust 之前只有 JSON。

这和前面 `search` 的情况很像：

- 默认 JSON 对自动化很好
- 但人在终端里临时看状态时，不够顺手

## 这次做了什么

Rust `status` 新增了：

- `--human`

开启后会打印 Python 风格的状态概览：

- `MemPalace Status — N drawers`
- `WING: ...`
- `ROOM: ...`

没有 palace 时，也会和 Python 一样打印：

- `No palace found at ...`
- `Run: mempalace init <dir> then mempalace mine <dir>`

## 一个实现点

这里不能直接靠 `Status.rooms` 来打印 Python 风格分组。

原因是 `Status.rooms` 在 Rust 里是**全局聚合**，不是“某个 wing 下的 rooms”。  
如果直接拿它来打印，会丢失 wing-room 层级关系。

所以 `status --human` 这里额外取了一次 taxonomy：

- `taxonomy.taxonomy[wing][room] = count`

这样输出时才能真正按：

- wing
- room

这两层来组织。

## 为什么默认仍然保留 JSON

和 `search` 一样，原则没变：

- 默认输出：稳定、机器可读
- `--human`：给真人看，强调可读性

如果把默认输出直接改成文本，会破坏已有测试和自动化调用。

## 这次补的回归

补了 3 条 CLI 回归：

- `status --help` 会提示 human 模式
- `status --human` 在 no-palace 场景下输出 Python 风格提示
- `status --human` 在有数据时输出 Python 风格 wing/room 区块

## 一个小知识点

当一个 summary 结构是“全局统计”时，不要勉强拿它去还原分层展示。

如果 UI / CLI 需要的是树状信息，就应该直接拿树状数据源。  
这里的 taxonomy 就比全局 rooms 计数更适合做人类可读输出。
