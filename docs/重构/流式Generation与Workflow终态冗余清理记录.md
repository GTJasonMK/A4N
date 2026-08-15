# 流式 Generation 与 Workflow 终态冗余清理记录

日期：2026-08-14

## 搜索起点与 BFS 覆盖范围

本专项从用户发起流式生成功能开始，按广度优先顺序闭合以下调用拓扑：

    Frontend 蓝图 / 灵感 / 章节 / Optimize 功能入口
    → frontend typed API / generation client / task store / event reducer
    → API contract GENERATION_START_ROUTES / command manifest / schema export
    → Web route / Tauri command / CLI command envelope
    → Runtime start_*_generation_typed
    → spawn_generation_execution
    → Orchestrator start_*_generation_task
    → reserve_generation_task / task_queue / generation checkpoint
    → acquire_generation_task_execution
    → GenerationTaskSession / publisher / checkpoint writer / terminal coordinator
    → Blueprint / Blueprint Refine / Inspiration / Chapter / Optimize Analysis / Chat / Patch worker
    → Generation completed / failed / cancelled / interrupted
    → Workflow completed / failed / cancelled

反向追踪了 `GenerationTaskExecution::finish/fail/fail_with_batch_summary`、
`GenerationTerminalCoordinator`、Generation event snapshot、PostgreSQL lifecycle 事务、
`generation_task_checkpoint`、`task_queue`、`workflow_run`、启动恢复和用户取消路径。
横向比较了七类绑定 Workflow 的流式 worker，并纳入 Outline、Video、Image、
Portrait 等只使用 Generation 终态的相邻路径，确认共享分类不会改变它们的生命周期。

显式依赖覆盖 Rust 调用、导入、`pub use`、trait/Host port、Serde 合同、
SeaORM 事务和测试；隐式依赖覆盖命令表、路由表、generation event schema、
`workflow_kind`、`active_task_id`、恢复扫描和前端终态投影。该链路不存在 XML DI、
反射、动态代理或 AOP 切面；运行时动态分派只来自显式命令表和 Host trait，均已纳入搜索闭包。

## 数据与语义证据

通过项目现有 `a4n-postgres` 容器对开发库做了只读查询，未修改数据：

- Generation checkpoint 中有 3 条 Blueprint failed、1 条 Inspiration failed，均来自真实 worker 运行。
- Workflow 中 Blueprint 为 12 completed / 3 failed，Inspiration 为 19 completed / 2 failed，Chapter 为 17 completed / 3 failed。
- 这些 failed Workflow 全部为 `can_resume=true`，与统一 Workflow 终态规则一致。
- 目标七类 Generation reservation 的 `workflow_run_id` 全部为空；Workflow 在 worker 内部后建立，因此不能直接依赖 Generation DB terminal 事务代写 Workflow 终态。
- 开发库没有目标七类的 interrupted 样本，本次没有从“样本为零”推导语义；而是沿用已有规范：用户取消或 `generation_interrupted` 对 Workflow 是可恢复 cancelled，Generation Task 自身保留 cancelled/interrupted 的精确区分。

因此没有强行改造 reservation 时序、篡改已发布的 Started event，也没有新增隐式绑定或兼容回退。当前最小且正确的唯一 owner 是编排层的共享 Generation/Workflow failure settle。

## 优化记录

1. 新增唯一 `generation_failure_is_cancelled_or_interrupted` 分类规则。
2. `GenerationTaskExecution::fail_with_batch_summary` 改为复用该规则，删除错误码分支的第二实现。
3. 新增唯一 `fail_generation_workflow_execution`，统一 failed/cancelled 映射、Workflow 写入和双错误合并。
4. 共享 settle 必须使用真实 Generation error message，未新增默认原因。
5. 删除 Blueprint 整套私有 failure settle 状态机。
6. 删除 Blueprint Refine 整套私有 failure settle 状态机。
7. 删除 Inspiration 整套私有 failure settle 状态机。
8. 删除 Optimize 私有 `is_generation_cancelled` 重复判定。
9. Optimize Analysis 只保留 Turn 与 command error 清理，终态委托共享 owner。
10. Optimize Chat 只保留 Turn 清理，删除两份 cancelled/interrupted 条件。
11. Optimize Patch 只保留 Turn 与 agent event 清理，终态委托共享 owner。
12. Chapter Generation 只保留章节 session 状态恢复，删除私有 Workflow outcome 映射。
13. 删除 Chapter Generation 重复 outcome 单测，由共享分类回归测试覆盖三种语义。
14. Blueprint 成功路径改用已有 `complete_workflow_run_validated`。
15. Blueprint Refine 成功路径改用同一 completed owner。
16. Inspiration 成功路径改用同一 completed owner。
17. Optimize Analysis 成功路径改用同一 completed owner。
18. Optimize Chat 成功路径改用同一 completed owner。
19. Optimize Patch 成功路径改用同一 completed owner。
20. 删除 `complete_chapter_generation_workflow` 窄转发 facade。
21. Chapter Generation 完成改用统一 validated owner。
22. Chapter Generation 开始不再直接调用原始 upsert，改用 `begin_workflow_run_validated`。
23. 删除 Outline 只包装 `Err(execution.fail(...))` 的单行 failure facade。
24. 删除退役的 Workflow outcome、Generation status 和 execution type 导入残留。

## 清理后唯一事实标准

    Generation worker 错误
    → generation_failure_is_cancelled_or_interrupted
       ├─ cancelling / generation_interrupted
       │  → Workflow cancelled + can_resume=true
       └─ 其他错误
          → Workflow failed + can_resume=true
    → GenerationTaskExecution::fail
       ├─ Cancelling → Generation Cancelled
       ├─ generation_interrupted → Generation Interrupted
       └─ 其他 → Generation Failed
    → 合并 Generation 终态写入错误与 Workflow 终态写入错误

领域私有失败处理只能保留真实业务清理（Chapter session、Optimize Turn/agent event），
不再自建 Generation/Workflow 分类或双错误合并。

## 集中验证

本批完成超过 10 项优化后统一验证。所有 Cargo 命令显式使用
`/home/fufu/Code/A4N/target`：

- `cargo fmt --all`：通过。
- `cargo check -p a4n-orchestrator -p a4n-runtime --all-targets`：通过。
- Generation failure 共享分类：1 项通过。
- Generation permit 终态投影：1 项通过。
- Chapter Generation workflow fingerprint：1 项通过。
- 七类 running Workflow 输入规则：7 项通过。
- Chapter Generation client RAG 合同：1 项通过。
- 合计 11 项相关回归测试通过。
- `git diff --check`：通过。
