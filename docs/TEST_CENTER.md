# 管理员功能测试中心

管理员可以从设置页打开独立的 `/admin/test-center`。测试中心包含项目操作场景，以及当前工作树 `scripts/tests/test_matrix.json` 明确登记的功能测试分组。分组由现有 `run_test_matrix.py` 执行，管理员只能选择已登记的分组 ID；页面不接收命令或参数化 shell 输入。

## 运行方式

- `测试副本`是默认目标。没有源项目时创建新的最小小说项目；指定源项目时复制它作为测试数据。
- `当前项目`需要管理员明确选择。写入场景会再次确认，测试产生的变化会保留。
- 同一时间只允许一个测试运行。运行状态和步骤会写入应用数据目录 `test-runs/<run_id>.json`。
- 项目测试分组始终从 Host 内置的矩阵目录显示；运行时如果当前安装包没有对应测试源码，会明确失败并说明原因。每个 suite 的运行状态、失败日志摘要和矩阵报告路径保存在测试运行详情。删除运行记录时一并删除它专属的矩阵报告。
- `全部功能测试`由矩阵中的 `functional_suites` 固定顺序组成；新增或移除测试入口时必须同时更新功能分组，矩阵检查会拒绝遗漏、重复、未知 suite 和覆盖不一致。
- Tauri 桌面组需要启动环境配置预构建应用路径 `A4N_WDIO_APP_BINARY`、隔离 `A4N_DATABASE_URL`、`A4N_QDRANT_URL` 和 `A4N_JWT_SECRET`。缺少前置配置时该组会明确失败。
- Host 重启时仍处于 `queued`/`running` 的记录会被标记为失败，不会伪造通过。
- 测试副本只能从测试运行记录删除；当前项目不能通过历史删除。

## 新增场景

场景注册表位于 `crates/host/a4n-runtime/src/actions/admin_records/test_console.rs`。新增场景时必须同时完成：

1. 管理员项目操作场景在 `scenarios()` 中增加唯一 ID、分类、影响级别、目标模式、参数 schema 和超时，并在 `run_scenario()` 中实现 handler。
2. 仓库功能测试在 `test_matrix.json` 中维护 suite 命令、功能分组和完整覆盖清单；通过矩阵 runner 返回每个 suite 的实际状态，不在 TestConsole 复制测试命令。
3. 新增任何场景时为前端补齐必要的中英文文案，并为参数校验、目标选择、结果状态和报告清理补充定向测试。

运行 `python3 scripts/tests/check_test_catalog.py` 检查场景 ID、handler、测试矩阵 profile 和前端 npm script 引用。

该门禁也检查聊天模型调用的执行器、恢复策略和功能套件登记。运行断言与真实模型业务评测的边界及入口见 [模型调用可靠性](model-call-reliability.zh.md)；登记检查通过不表示运行覆盖率或真实模型质量达标。

## 测试层职责

- 测试中心：管理员手动触发的真实项目功能场景，以及测试矩阵中经过筛选、归类的 Rust、前端、Python、浏览器、创作端到端和 Tauri 功能 suite。
- `scripts/tests/test_matrix.json`：所有自动化检查的维护目录；测试中心只呈现 `test_center.functional_suites`，不把格式、静态边界、构建和其他非功能覆盖保障作为功能测试。
- `scripts/tests/check_test_catalog.py`：维护门禁，加入 fast profile，阻止未注册或失效入口继续存在。
- 浏览器/Tauri E2E：使用既有隔离 runner 启动；Tauri 桌面流程必须由启动 Host 的环境明确提供隔离数据库和预构建应用路径。

运行历史元数据和步骤默认保留在应用数据目录；删除记录时可以选择同时删除测试中心创建的测试项目。
