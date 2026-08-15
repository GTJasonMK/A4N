# 通用 Workflow 取消与 Canvas 节点生命周期冗余清理记录

日期：2026-08-14

## 搜索起点与 BFS 覆盖范围

本专项从既有失败
canvas::editing_nodes::tests::trim_video_cancellation_marks_spec_and_checkpoint_cancelled
出发，按广度优先顺序追踪取消请求、节点生命周期、持久化状态与异步视频执行：

    Canvas TrimVideoNode 取消
    → CanvasEditingNodeHostPort::trim_video
    → run_canvas_editing_node_validated
    → start_canvas_node_execution / finish_canvas_node_error
    → Canvas 节点 Workflow / Checkpoint / generation_spec
    → Canvas 整图 run 与节点间取消检查
    → Canvas 视频 submit / result / polling 取消检查
    → Runtime cancel_workflow
    → task_id / workflow_run_id 反查与权限校验
    → Orchestrator cancel_workflow_run_and_interrupt_videos
    → DB cancel_workflow_run_persistence 事务
    → workflow_run / workflow_checkpoint / active_task_id
    → TaskHandle / PersistentTaskQueue / generated_video 中断

继续横向比较了以下活跃路径：

    独立 Canvas 节点与共享整图节点的启动、成功、失败、取消
    整图节点执行前、节点间、执行后及 Host 错误后的取消竞争
    Canvas 视频提交前、provider 结果回写前、轮询中及迟到结果时的取消
    按 task_id、按 workflow_run_id、同时提供两者及无持久记录任务的取消
    删除生成视频时对关联 provider Workflow 的取消
    provider completed / failed / interrupted 状态向 Canvas Workflow 的同步

显式依赖覆盖 Rust 函数调用、模块导入、pub use、Host trait、Runtime action、
API contract、Server 字符串路由、前端 cancelWorkflow、SeaORM 查询和事务。隐式依赖
覆盖 command manifest、Serde 严格输入、认证用户注入、task_queue.workflow_run_id、
workflow_run.active_task_id、Canvas 视频 metadata、CancellationToken 与 provider job
恢复状态。该链路不存在 XML DI、反射调用、动态代理或 AOP 配置；动态分派仅来自显式
Host trait、命令表和 provider 状态，均已纳入搜索闭包。

## 生产数据证据

通过健康的 a4n-postgres 容器对开发库执行只读事务：

    cancelled_workflow_total=9
    cancelled_with_active_checkpoint=0
    cancelled_with_active_task_id=0
    active_task_workflow_owner_mismatch=0

结论：

- 已取消 Workflow 没有残留非终态 Checkpoint。
- 已取消 Workflow 没有残留 active_task_id。
- 活跃持久任务与 Workflow 的活动任务 owner 没有分叉。
- DB 取消事务已经完整拥有 Workflow、活跃 Checkpoint 和活动任务解绑，不需要节点返回后再写一次取消 Checkpoint。

取消原因分布还发现一条 chapter_shooting_script Workflow 被通用入口写成
“用户取消画布运行”，证明 Runtime 通用取消曾错误携带 Canvas 专属文案；其余现有原因
包括“任务已取消”和真实 provider 错误。清理后通用入口只写“用户取消工作流”，Canvas
摘要读取持久化原因，不再制造第二份文案。

## 优化记录

1. 将节点启动不变量统一为“调用 Host 前必须已有 running Checkpoint”。
2. 独立节点与共享整图节点复用同一 running Checkpoint 创建路径，删除独立节点缺失 Checkpoint 的生命周期分叉。
3. 确立 cancel_workflow_run_persistence 为 Workflow 与活跃 Checkpoint 取消状态的唯一数据库 owner。
4. 删除节点返回后的第二次 cancelled Checkpoint 写入，消除终态写门禁冲突。
5. 删除单调用 cancel_canvas_node。
6. 删除取消节点状态错误聚合器；取消后只同步不属于 Workflow 事务的 generation_spec。
7. Canvas 节点取消摘要改为读取 Workflow 持久化 error_message。
8. cancelled Workflow 缺少非空原因时显式返回 InvalidState，不再生成默认文案。
9. Canvas 整图在节点前、节点间、节点后和 Host 错误后统一读取同一持久化取消原因。
10. 删除整图摘要中硬编码的“用户取消画布运行”。
11. Canvas 视频提交前、结果写入前和轮询中的取消检查复用同一原因查询。
12. Runtime 通用取消原因收敛为私有常量“用户取消工作流”。
13. 删除只转发一次的 cancel_authorized_workflow_run。
14. 删除 cancelled_workflow_ids 第二套取消结果集合。
15. 删除 task_cancel_signal_allowed 布尔状态机及只验证该状态机的弱单测。
16. 关联 Workflow 的任务取消只调用 Workflow owner；TaskHandle、队列任务和关联视频由该 owner 统一中断。
17. 无关联 Workflow 的普通任务仍由 Runtime 直接取消 TaskHandle 与队列记录，保留真实独立场景。
18. 删除原样返回布尔参数的 canvas_node_should_finalize_on_success。
19. 删除固定返回 Completed 的 canvas_node_success_finalize_outcome。
20. 删除仅被自身测试消费的 canvas_workflow_checkpoint_is_terminal。
21. 删除单调用 CanvasVideoWorkflowStatusAction、状态映射函数及其重复测试。
22. 删除单调用 complete_canvas_node_workflow_validated facade，节点成功 owner 直接完成独立 Workflow。
23. 删除单调用 finalize_canvas_video_workflow_validated facade，provider 同步 owner 直接推进 Canvas Workflow。
24. Canvas provider 同步只接受生产调用实际产生的 completed、failed、interrupted；其他状态在写 Checkpoint 前显式失败。
25. interrupted Canvas Workflow 明确保留 can_resume=true；completed/failed 继续复用通用终态 owner。
26. failed Canvas 视频缺少非空错误原因时显式失败，删除“画布视频生成失败”默认值。
27. DB 取消原因改用既有必填文本校验，删除空原因自动替换为“工作流已取消”的 fallback。
28. 节点 Checkpoint 状态删除未知值通用文案分支，只接受真实生产状态。
29. 取消回归测试改为调用真实 DB 取消事务，不再手工只修改 Workflow 状态制造不完整夹具。
30. 新增一个最小数据库回归测试，覆盖 Canvas provider 三种真实同步状态及未知状态写前拒绝。
31. 删除全部退役导出、导入和符号残留。

## 清理后唯一事实标准

    节点启动
    → start_canvas_node_execution
    → running Workflow + running Checkpoint
    → Host 执行

    用户取消
    → Runtime 权限校验与目标解析
    → cancel_workflow_run_and_interrupt_videos
    → cancel_workflow_run_persistence 单事务
       ├─ Workflow = cancelled
       ├─ 活跃 Checkpoint = cancelled
       ├─ active_task_id = NULL
       └─ 持久化唯一取消原因
    → Orchestrator 中断 TaskHandle / queue / generated_video

    节点观察到取消
    → 读取持久化取消原因
    → 同步 generation_spec
    → 返回 cancelled 摘要
    → 不再写 Workflow 或 Checkpoint

    Canvas provider 结果
    → sync_canvas_video_workflow
    → completed / failed / interrupted 唯一状态推进

Runtime 仍负责 transport 目标解析和权限；Orchestrator 负责进程内中断；DB 事务负责持久化
权威状态。三者职责不同，没有保留兼容别名、回退写入或第二状态机。

## 集中验证

本批超过 10 项优化后统一验证。所有 Cargo 命令显式复用
/home/fufu/Code/A4N/target/；数据库测试读取 docs/TEST_DATABASE.md 并加载仓库
.env 中隔离的 A4N_TEST_DATABASE_URL。

- a4n-db 取消原因目标测试：1 项通过。
- Canvas editing：7 项通过，包含原失败的裁剪取消回归。
- Canvas image generation：5 项通过。
- Canvas text generation：7 项通过。
- Canvas output nodes：6 项通过。
- Canvas probe nodes：4 项通过。
- Canvas graph execution：6 项通过。
- Canvas video generation 规则：3 项通过。
- Canvas provider Workflow 状态同步：1 项通过。
- Runtime task records：3 项通过。
- 合计 43 项目标测试通过。
- cargo check -p a4n-db -p a4n-orchestrator -p a4n-runtime --all-targets：通过。
- cargo fmt --all -- --check：通过。
- git diff --check：通过。

Runtime 测试二进制首次链接耗时 68 秒，因此先在 180 秒构建上限内完成 --no-run，
随后实际 3 项测试在 60 秒测试上限内于 0 秒完成；没有放宽测试执行时限。
