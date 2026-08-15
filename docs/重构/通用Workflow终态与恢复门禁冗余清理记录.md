# 通用 Workflow 终态与恢复门禁冗余清理记录

日期：2026-08-14

## 搜索起点与 BFS 覆盖范围

本专项分别从“领域执行结果落终态”和“用户恢复 Workflow”两个生产功能入口开始，按广度优先顺序闭合调用拓扑：

    领域功能入口
    → begin_workflow_run_validated / begin_or_resume_workflow_run_validated
    → 领域 execute Result<T, AppError>
    → 领域自建 settle / finalize_workflow_result / finalize_workflow_outcome_result
    → finalize_workflow_run_outcome_validated
    → OrchestratorContext::mark_workflow_status
    → WorkflowApplication
    → a4n_db::workflow::mark_workflow_status
    → workflow_run.status / can_resume / resume_block_reason / error_message / completed_at

    CLI / Web / Desktop 的 resume_workflow_run
    → API contract 与 command manifest
    → Server 字符串路由 / Host action
    → Runtime resume_workflow_run_for_user
    → 权限与作用域校验
    → plan_unified_resume
    → Video provider / Canvas graph / Chapter analysis / Chapter content sync
    → mark_workflow_running_validated
    → a4n_db::workflow::resume_workflow_run

    视频 provider 结果与轮询
    → generated_video 状态
    → mark_video_provider_workflow_validated / sync_canvas_video_workflow
    → provider Workflow / Canvas Workflow / Checkpoint
    → completed / failed / interrupted

横向比较了 Chapter Analysis、Chapter Content Sync、Chapter Scene、Character Discovery、
Character Profile Sync、Character Conflict、Implicit Review、Object Discovery、Galgame、Manga、
Source Sync、Portrait、Scene Profile、Outline、Import Reverse、Image Generation、Batch Image、
Video Prompt Generation、Video Prompt Optimization、Inserted Segment、Shooting Script 等活跃路径。

显式依赖覆盖 Rust 调用、导入、pub use、SeaORM 事务、错误类型和测试夹具；隐式依赖覆盖
API contract、command manifest、Server 命令表、CLI 命令字符串、前端 resumeWorkflowRun、
Serde 输入、workflow_kind 分发、scope_table/scope_id、任务权限和 provider metadata。该链路
不存在 XML DI、反射、动态代理或 AOP 配置；运行时动态分派只来自显式命令表、Host trait 和
workflow_kind，均已纳入搜索闭包。

## 生产数据证据

对开发库执行只读查询得到：

    cancelled | can_resume=false = 2
    cancelled | can_resume=true  = 7
    completed | can_resume=false = 364
    failed | can_resume=true     = 58
    interrupted | can_resume=true = 2
    superseded | can_resume=false = 5

同时确认：

- failed、cancelled、interrupted 缺少 error_message 的记录为 0。
- “工作流失败”“工作流未完成”“工作流已取消”等默认原因命中为 0。
- can_resume=true 且仍带 resume_block_reason 的记录为 0。
- completed 缺少 resume_block_reason 的 364 条记录符合“状态本身即可阻止恢复”的语义。
- cancelled 与 superseded 的不可恢复记录均已有明确阻止原因。

因此没有统一篡改 cancelled 的 can_resume：用户显式取消保持 false；生成过程内部取消或中断
保持 true。清理只删除默认原因和重复状态机，不改变这两种真实生产语义。

## 优化记录

1. 保留 finalize_workflow_result 作为 Result settle 的唯一 owner。
2. 将 settle context 改为必填领域名称，所有成功写终态失败和双错误使用同一合并规则。
3. 删除与其主体重复的 finalize_workflow_outcome_result。
4. 删除布尔 success 终态 facade finalize_workflow_run_validated。
5. 删除 workflow_run_finalize_status_from_success 布尔映射。
6. Batch Image 改为显式 complete/fail/cancel owner，不再走布尔终态分叉。
7. failed/cancelled 便捷入口改为必填非空原因参数。
8. finalize_workflow_run_outcome_validated 删除“工作流未完成”默认原因。
9. failed/cancelled 缺少或只含空白原因时在编排边界显式失败。
10. DB upsert 边界要求 failed/cancelled/interrupted 必须有 error_message。
11. DB upsert 边界要求非恢复且非 completed/running 的状态必须有 resume_block_reason。
12. DB 状态字段禁止 completed/superseded 标记 can_resume=true。
13. DB mark_workflow_status 复用同一状态字段校验，封堵绕过编排层的写入路径。
14. DB 状态写入只保存规范化后的真实原因，不再从空值生成原因。
15. 恢复门禁加入 status，completed 直接返回“工作流已完成，不能恢复”。
16. running 直接拒绝重复恢复，不再依赖 can_resume 布尔值。
17. can_resume=true 只允许 failed/cancelled/interrupted 三种真实恢复状态。
18. 其他 can_resume=false 状态必须返回持久化原因；缺失时显式报告数据不变量错误。
19. DB resume_workflow_run 删除 running 幂等成功路径，重复恢复改为 StatusConflict。
20. Chapter Content Sync 删除自建 can_resume fallback，复用统一恢复门禁。
21. Chapter Analysis 两个恢复入口删除各自默认文案，复用统一恢复门禁。
22. Chapter Analysis ResumeBlockReason 删除“当前不可继续同步”默认原因，缺失原因显式暴露状态不变量。
23. 删除 Chapter Analysis 自建 Result settle 状态机，三个调用点改用唯一 owner。
24. 删除 Galgame 自建 Result settle 状态机。
25. 删除 Source Sync 自建 Result settle 状态机。
26. 删除 Portrait 自建 Result settle helper，提示词、单图、变体和正常批量路径改用唯一 owner。
27. Portrait 中断或全失败批次要求聚合结果携带真实 first_error，不再允许空原因落终态。
28. 删除 Scene Profile status.rs 整个重复终态模块，String 错误先转换为 AppError 后统一 settle。
29. 删除 Video Prompt Generation 自建成功/失败/取消终态状态机。
30. Inserted Segment 删除成功和失败两条手工终态分支，统一 settle 后再报告完成进度。
31. Chapter Shooting Script 单元完成与错误路径统一使用 Result settle；Partial 继续保留真实 interrupted 语义。
32. Chapter Scene 与 Character Conflict 原先无 context 的 settle 改为显式领域 context。
33. Object Discovery、Character Profile Sync、Implicit Review、Character Discovery、Video Prompt Optimization、Image Generation、Chapter Content Sync 等既有调用统一为领域 context。
34. Outline、Import Reverse、Manga、Shooting Script 四个旧 helper 调用迁移到唯一 Result settle。
35. 删除 provider 可接受任意 status/can_resume/error_message 的伪通用构造器。
36. 删除只转发该构造器的 begin_video_provider_workflow_validated facade。
37. Provider 新建 Workflow 只剩真实使用的 RUNNING 输入构造器。
38. 删除 provider 终态阻止原因默认值，因为终态不再经该伪通用输入创建。
39. Canvas provider failed/interrupted 同步必须携带真实错误原因，缺失时写前失败。
40. 删除 Canvas graph 完成 facade 内与通用终态 owner 重复的 workflow_run_id 校验。
41. 更新 cancelled + None 的旧数据库夹具，改为真实取消原因。
42. 增加重复恢复 running Workflow 的数据库回归断言。
43. 删除全部退役导出、导入、函数和 Scene Profile status 模块残留。

## 清理后唯一事实标准

    普通领域执行
    → Result<T, AppError>
    → finalize_workflow_result(context)
    → Completed / Failed / Cancelled
    → finalize_workflow_run_outcome_validated
    → DB mark_workflow_status

    特殊部分完成业务
    → 领域 owner 判定 Partial / Interrupted / Aggregate outcome
    → complete_workflow_run_validated
       或 fail_workflow_run_validated(real_reason)
       或 cancel_workflow_run_status_validated(real_reason)

    统一恢复
    → Runtime 加载 run 并完成权限校验
    → plan_unified_resume(status, can_resume, persisted_reason, kind, scope)
    → 只允许 failed / cancelled / interrupted
    → DB resume_workflow_run 拒绝 running 重复恢复

    终态持久化边界
    → failed / cancelled / interrupted 必须有 error_message
    → completed / superseded 必须 can_resume=false
    → 非恢复且非 completed/running 必须有 resume_block_reason
    → 不生成默认原因

同一业务结果只有 finalize_workflow_result 一套 settle；特殊 Aggregate/Partial 只保留其真实领域判定，
最终状态仍进入同一 complete/fail/cancel owner。未增加兼容别名、回退、占位或第二来源。

## 集中验证

本批超过 10 项优化后统一验证。所有 Cargo 命令显式使用
/home/fufu/Code/A4N/target；数据库测试先读取 docs/TEST_DATABASE.md，并加载仓库 .env 中隔离的
A4N_TEST_DATABASE_URL，全部串行执行。

- a4n-db Workflow 规则：24 项通过。
- a4n-db 视频断点与重复恢复：1 项通过。
- a4n-db generation_persistence：4 项通过。
- Workflow 终态与恢复纯规则：8 项通过。
- Canvas provider Workflow 同步：1 项通过。
- Portrait Application：2 项通过。
- Scene Profile Application：14 项通过。
- Source Sync：12 项通过。
- Galgame：17 项通过。
- Chapter Analysis Workflow：13 项通过。
- Runtime task records：3 项通过。
- Shooting Script 相关：25 项通过。
- Canvas graph execution：6 项通过。
- 合计 130 项唯一目标测试通过。
- cargo check -p a4n-db -p a4n-orchestrator -p a4n-runtime --all-targets：通过。
- cargo fmt --all：通过。
- git diff --check：通过。

Runtime 测试第一次在 60 秒上限内完成了测试二进制编译但尚未执行，命令按 timeout 返回 124；
复用同一 target 缓存后仅补跑 Runtime 组，3 项测试在 60 秒执行上限内全部通过，没有重跑已通过组。
