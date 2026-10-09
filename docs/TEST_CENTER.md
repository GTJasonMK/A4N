# 管理员功能测试中心

管理员可以从设置页打开独立的 `/admin/test-center`。测试中心运行注册过的功能场景，使用真实业务 application port 和项目数据，不执行任意 shell、cargo 或 npm 命令。

## 运行方式

- `测试副本`是默认目标。没有源项目时创建新的最小小说项目；指定源项目时复制它作为测试数据。
- `当前项目`需要管理员明确选择。写入场景会再次确认，测试产生的变化会保留。
- 同一时间只允许一个测试运行。运行状态和步骤会写入应用数据目录 `test-runs/<run_id>.json`。
- Host 重启时仍处于 `queued`/`running` 的记录会被标记为失败，不会伪造通过。
- 测试副本只能从测试运行记录删除；当前项目不能通过历史删除。

## 新增场景

场景注册表位于 `crates/host/a4n-runtime/src/actions/admin_records/test_console.rs`。新增场景时必须同时完成：

1. 在 `scenarios()` 中增加唯一 ID、分类、影响级别、目标模式、参数 schema 和超时。
2. 在 `run_scenario()` 中增加同名 handler，并为每个实际操作记录步骤和影响。
3. 为前端增加场景标题和描述的中英文文案（如果需要显示本地化文本）。
4. 为成功、失败、参数拒绝和目标校验补充 Rust 定向测试。

运行 `python3 scripts/tests/check_test_catalog.py` 检查场景 ID、handler、测试矩阵 profile 和前端 npm script 引用。

## 测试层职责

- 测试中心：管理员手动触发的真实功能场景和项目变化。
- `scripts/tests/test_matrix.json`：Rust、前端、Python、合同、浏览器和 nightly 编排。
- `scripts/tests/check_test_catalog.py`：维护门禁，加入 fast profile，阻止未注册或失效入口继续存在。
- 浏览器/Tauri E2E：继续在隔离环境或 CI 中运行，不从已安装应用直接启动工具链。

运行历史元数据和步骤默认保留在应用数据目录；删除记录时可以选择同时删除测试中心创建的测试项目。
