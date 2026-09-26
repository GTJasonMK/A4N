# 第三轮清理与收尾

2026-09-26，已直接在 `debug` 完成。修改限定于前轮模型调用优化、端到端入口及相关 UI 状态；保留其他会话 WIP，未创建分支、整体提交或合并。

## 清理和修复

- [Agent 恢复入口](../../crates/agent/a4n-agent-runtime/src/engine/model.rs)：共用原始请求快照读取和响应恢复。缺少或无效快照时明确报错，移除猜测原生协议的回退；重放保留工具推理字段及厂商不透明状态。删除主循环的第二套响应解析和整份请求重复序列化。
- [视频派生字段](../../crates/host/a4n-orchestrator/src/video/prompts/json/derived_fields/shot.rs)：删除两个相同的结尾摘要函数、单次调用的时间块转发函数及无用参数，直接读取已有规划字段，保持现有输出合同。
- [浏览器公共助手](../../frontend/e2e/full-project/user-flow.mjs)：删除两个无调用操作、无用导出/导入和重复等待；关闭窗口不再先空等 30 秒并吞掉错误。仅容忍确实已替换的 DOM 节点，其他异常保留。字面文本断言使用 `includes`，移除错误的正则转义复用。
- [状态栏](../../frontend/src/components/ui/StatusBar.tsx)：持续状态的三个调用方均为文本，将接口收敛到字符串，取消无调用的 ReactNode 扩展面。
- [一键入口](../../scripts/tests/run_creation_e2e.py)：浏览器直接访问 Server 提供的生产构建，删除额外 Vite 进程、代理和端口。构建明确使用 Web 生产配置；实际网络记录包含 124 个生产资源，没有开发源码或 Vite 请求。
- 二进制指纹覆盖 crate 内资源、ONNX 版本、构建脚本、Cargo/工具链配置；前端指纹覆盖公共资源。文件路径和内容哈希分界明确，避免拼接歧义。构建读取冻结快照内的脚本；复制快照时跳过依赖、构建产物及环境文件。
- 修复测试误报：CLI 进程非零退出时整轮必须失败，即使它已写出某阶段的通过记录。缺失/损坏的报告记为该组失败，继续其他独立组；不再由报告解析异常中止后续检查。
- [进程清理](../../scripts/tests/run_browser_full_project_e2e.py)：终止失败不再静默忽略，日志句柄仍关闭，清理错误计入结果并继续处理其他服务。即使 PostgreSQL 日志无法打开，也会执行容器回收；源码版本核对异常不能阻止最终报告落盘。
- [创作 CLI 检查](../../scripts/tests/run_authoring_stage_cli_e2e.py)：旧版本写入必须返回具体版本冲突原因，移除“任意错误都算拒绝成功”的分支；任务观察超时不能冒充预期失败。

当前仍在使用的显式 JSON 工具协议、纯环境镜头、无蓝图时的正式资料路径和 SSE 快照恢复继续保留。缺失原始请求、真正的调用错误和未完成任务都不会被写成成功。

## 验证

| 检查 | 结果 |
| --- | --- |
| Agent Runtime 单测 | 32 通过，包含快照缺失、工具推理及不透明元数据恢复 |
| 视频提示词 JSON/派生字段单测 | 44 通过 |
| 验收门禁、指纹和进程清理单测 | 8 通过；包括完整 E2E 后补充的 PostgreSQL 日志失败回收检查 |
| 浏览器助手和诊断反向检查 | 2 通过 |
| 实际 CLI 五组业务及故障检查 | 69 通过（14/19/8/17/11） |
| 生产前端真实 Chromium | 九阶段全过，含 390px 编辑、图片解码、视频采用与刷新后完整播放 6 秒 |
| Server/CLI/Worker/迁移构建、前端类型检查及生产构建 | 通过 |
| 全仓 rustfmt、git diff --check | 通过 |

通过报告：[cleanup-20260926-second/report.json](../../data/creation-e2e/cleanup-20260926-second/report.json)。本次记录 627 次 Provider 请求，`success: true`、浏览器退出 0、运行器失败列表为空，结束时 `matches_current_worktree: true`。

后端单测和清理前副本在 [final-sweep](../../data/prompt-context-audit-20260926/final-sweep/)。首次全链构建中断、没有完成回执，保留 [interrupted.json](../../data/creation-e2e/cleanup-20260926-first/interrupted.json)，未计作通过；随后沿用保存的源码重新构建并完整通过。

本轮服务、容器、浏览器均已停止，共享 target 在独占锁内完成清理。测试数据和证据保留。当前改动仍未提交。

## 使用及边界

```bash
python3 scripts/tests/run_creation_e2e.py \
  --postgres-image docker.m.daocloud.io/library/postgres:16
```

旧二进制回执漏记了部分输入，不能由新指纹规则直接复用，需要重新构建。缺少原始请求快照的历史模型执行会明确报错，不再猜测协议。Mock 验证编排、状态、协议和持久化，不代表真实模型内容质量，也不保证全部未知场景无错误。
