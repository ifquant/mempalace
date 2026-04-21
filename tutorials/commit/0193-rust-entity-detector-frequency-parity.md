# Commit 0193: Rust entity detector frequency parity

## 背景

继续从 parity ledger 的 `Deeper non-CLI behavior audit` 往下盘，选中 `entity_detector` 这块底层行为。

Python `entity_detector.py` 的候选提取规则是：实体名必须至少出现 3 次才进入后续评分。

```python
return {name: count for name, count in counts.items() if count >= 3}
```

Rust 此前 2 次出现就会进入评分。只要两次都带有较强 person/project signal，就可能在 onboarding/init 里被自动写入 registry，形成 Python 不会自动确认的实体。

## 主要目标

- 让 Rust entity detector 的候选频率阈值对齐 Python。
- 避免 2 次出现的强 signal 名字被自动收进 people/projects。
- 保持已覆盖的正常 3+ 次实体检测不变。

## 改动概览

- 更新 `rust/src/entity_detector.rs`。
- 将 frequency gate 从 `< 2` 改为 `< 3`。
- 新增 `entity_detector_requires_python_candidate_frequency` 测试，确认只出现两次的 `Jordan` / `Atlas` 不会被检测为实体。
- 更新 `rust/src/bootstrap.rs` 的 bootstrap fixture，让正向检测样例满足 3 次出现门槛。
- 更新 `rust/tests/cli_integration.rs` 中依赖自动检测的 init/onboarding/registry fixtures，让它们继续验证正向路径，而不是依赖旧的 2 次候选门槛。
- 更新 `rust/tests/mcp_integration.rs` 中依赖自动检测的 project bootstrap / registry fixtures，让 MCP 覆盖也显式满足 3 次出现门槛。
- 更新 `rust/tests/service_integration.rs` 中依赖自动检测的 init / registry fixtures，让 service-level 覆盖也显式满足 3 次出现门槛。

## 关键知识

entity detection 的 false positive 成本比 false negative 更高。这个检测结果会进入 onboarding/bootstrap 结果，并可能写入项目本地 registry。

Python 用 3 次出现作为候选门槛，是为了过滤偶然提到的名字。Rust 如果 2 次就接收，会让“看起来更聪明”的检测结果和 Python 行为分叉。

## 补充知识

这次只对齐候选频率门槛。没有同时处理多词 proper noun、prose/readable extension 列表、交互确认流程或 Rust-only onboarding 输出面；这些如果要继续审计，应作为独立切片。

## 验证

- `cargo fmt --check`
- `cargo test entity_detector::tests::entity_detector_requires_python_candidate_frequency -- --exact`
- `cargo test bootstrap::tests::bootstrap_detects_rooms_and_entities_and_writes_files -- --exact`
- `cargo test cli_init_writes_entities_json_when_detection_finds_names --test cli_integration -- --exact`
- `cargo test cli_registry_summary_lookup_learn_and_research_work --test cli_integration -- --exact`
- `cargo test mcp_registry_tools_work --test mcp_integration -- --exact`
- `cargo test init_project_bootstraps_rooms_and_entities --test service_integration -- --exact`
- `cargo test registry_summary_lookup_and_learn_work --test service_integration -- --exact`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`

## 未覆盖项

- 未修改 Python 实现。
- 未修改 README 或 parity ledger。
- 未改变 onboarding CLI/MCP schema、registry、split、normalize 或 maintenance 能力面。
