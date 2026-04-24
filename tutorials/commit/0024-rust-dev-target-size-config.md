# 0024 压小 Rust dev/test 构建体积

这次改动很小，但很实用：给 `rust/` 加了一个本地 `.cargo/config.toml`，把开发期和测试期的默认构建体积先压下来。

## 做了什么

新增文件：

- `rust/.cargo/config.toml`

内容很简单：

```toml
[build]
incremental = false

[profile.dev]
debug = 0
codegen-units = 16

[profile.test]
debug = 0
codegen-units = 16
```

## 为什么这样做

前面已经看到这个仓库的 `rust/target` 很大，主要是：

- `debug/deps`
- `debug/incremental`
- `lancedb / lance / datafusion` 依赖链

这次先做的是“低风险、立刻见效”的收缩：

1. 关闭 `incremental`
   避免 `target/debug/incremental` 持续膨胀
2. 把 `dev/test` 的 `debug` 符号关掉
   直接缩小本地调试和测试产物
3. 保持 `release` 不变
   不影响以后真正要发版时的优化路线

## 测试

跑了最低成本验证：

```bash
cd rust && cargo check
```

## 新手知识点

Rust 的 `target` 变大，常见不是因为你的主程序真的有那么大，而是因为：

- 依赖很多
- debug 符号很重
- incremental 留下很多历史产物
- test binary 会把依赖再链接一遍

所以“减 target 体积”通常分两层：

1. 先减 **构建配置** 带来的膨胀
2. 再看是否需要减 **依赖本身**

这次只做第 1 层，因为它风险最低、收益最快。
