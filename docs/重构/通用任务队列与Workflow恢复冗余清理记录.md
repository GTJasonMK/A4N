# 通用任务队列与 Workflow 恢复冗余清理记录

## 搜索覆盖范围

本专项从动态 LLM 流、图片/视频 provider 请求、后台下载、Source Sync、Canvas 节点和管理端取消功能出发，按 BFS 追踪：

```text
Runtime provider / 业务调用
→ OrchestratorContext / DynamicProviderFacade
→ submit / submitted guard / acquire
→ MemoryTaskQueue waiting / semaphore / cancellation token / permit
→ running / completed / failed / cancelled 持久化
→ workflow_run.active_task_id 同步
→ 用户任务取消 / Workflow 取消
→ PersistentTaskQueue 启动恢复
→ stale task / stale workflow / provider 前 stale video 恢复
→ durable video_provider_job 恢复
→ chapter_analysis checkpoint resume
```

横向比较了 embedded LLM stream、sidecar LLM stream、图片生成、立绘、视频提交、视频轮询、Embedding 下载、Source Sync、Canvas 文本与媒体节点的队列生命周期，并核对 provider `Result<T, String>` 原始错误文本合同。隐式依赖覆盖 Rust trait 实现、Host facade、Tauri/Web 命令入口、`task_queue.workflow_run_id` 关联、Workflow 统一恢复分发、PostgreSQL 行锁、启动构造、durable provider job 与章节分析 checkpoint。项目无 XML DI、反射、动态代理或 AOP 配置。

## 优化记录

1. 将 `task_queue` 状态与 `workflow_run.active_task_id` 同步收敛到同一数据库事务。
2. 删除 Orchestrator 中事务外的第二次 Workflow 状态同步。
3. 删除 `status/workflow.rs` 单调用包装文件。
4. 删除 a4n-db 中重复公开的 `sync_workflow_active_task_for_queue_status` 状态写入口。
5. Drop 取消清理改为复用同一原子状态更新，补齐原先遗漏的 Workflow 同步。
6. 新增最小 `RunningTask` RAII 生命周期对象，统一 permit 与持久终态结算。
7. 非流式 `run_tracked` 改为复用 `RunningTask::finish`，删除独立 outcome 状态机。
8. 删除 `execution/outcome.rs` 重复完成/失败实现。
9. embedded LLM stream 删除手工 submit、guard、acquire、cancel 流程。
10. sidecar LLM stream 删除同一套手工生命周期分叉。
11. 两类 LLM stream 的完成、失败、建立失败和提前 Drop 均通过 `RunningTask` 结算。
12. `DynamicProviderFacade` 删除 submit、guard、acquire、cancel、mark done、mark failed 六个窄转发入口，只保留 stream 所需的单一 start 入口。
13. `OrchestratorContext` 删除无调用的 queue submit/guard/acquire/mark 公开转发。
14. 图片、立绘、视频提交和视频轮询删除整套手工队列生命周期，复用现有 tracked 执行入口。
15. 补齐 workflow context + cancellation 的 tracked 入口，避免 provider helper 再实现一次状态机。
16. 删除公开但仅测试使用的 `submit_with_workflow_context` 别名，测试直接验证内部持久提交合同。
17. 启动恢复将 `RecoverableQueuedTask` DTO 收敛为实际只需的 task id 列表。
18. 删除 recovery `task.rs` 单调用包装文件。
19. 删除“每个 stale task 单独恢复 Workflow”与“随后全量恢复 stale Workflow”的双重状态机。
20. 启动恢复现在先统一中断非 generation queue task，再由唯一的 stale Workflow 扫描决定恢复能力。
21. 修正 `character_profile_sync` 恢复测试输入：生产创建合同本身声明 `can_resume=true`，测试不再依赖旧的无条件兜底。
22. queue 与 generation lifecycle 继续使用同一行锁顺序：先 task row，后 workflow row，避免并行状态 owner。
23. 关联 `workflow_run` 缺失时不再静默成功，队列状态事务显式失败并回滚。
24. task row 持久化失败时立即撤销刚创建的内存 pending task，避免内存/数据库分叉。
25. 删除整个单实现 `a4n_queue::TaskQueue` trait，而不只删除 `PersistentTaskQueue` 的实现。
26. 删除 trait 附带的 submit/acquire-by-kind/status/history/cancel 分叉，以及 Orchestrator 的 `trait_impl.rs` 目录。
27. 删除按 task kind 获取任意任务的 acquire 路径和 `pop_matching_task`；生产调度只允许按持久 task id 获取许可。
28. 删除 `MemoryTaskQueue` 自行生成 UUID、通用 submit/status/history、`new/default` 等第二套生命周期能力。
29. UUID 生成收敛到持久提交 owner；`a4n-queue` 删除 `uuid` 与 `async-trait` 依赖。
30. `queue_stats` 改为 `MemoryTaskQueue` 固有只读能力，不再依附已删除 trait。
31. 删除 `PersistentTaskQueue::status/history/mark_done/mark_failed` 等仅测试或窄转发 API，测试直接查询数据库合同。
32. acquire、start 与 submitted guard 收窄到真实 crate/module 调用范围，不再暴露无生产消费者的接口。
33. 删除 `insert_task_queue_record_if_absent` 静默幂等写入；重复 task id 由数据库明确报错。
34. 展平 `PersistTaskQueueRecordInput`，删除只负责字段转发的 `PersistTaskFamilyContext`。
35. 删除 `persistence/submission/record.rs` 单调用映射包装，持久记录在唯一提交 owner 构造。
36. task row 写入失败时撤销内存 pending 投影，防止内存与数据库形成双事实来源。
37. `chapter_analysis` 创建时直接持久化 `can_resume=true`，恢复不再按 workflow kind 临时特判。
38. durable `video_provider_job` 恢复测试使用生产一致的 `can_resume=true`，并验证 queue task 中断、Workflow 保持 RUNNING、`active_task_id` 原子清空。
39. `RunningTask::finish_message` 保留 provider 原有 `Result<T, String>` 错误文本，不引入新的错误变体或兼容转换。
40. 删除 259 行依赖旧 trait 的内存队列测试，仅保留一个直接验证“持久任务按优先级、按 id 获取”的最小回归测试。
41. 删除两个只断言刚赋值字段的 `PersistTaskQueueRecordInput` 无价值测试。

## 清理后唯一事实标准

```text
非 Generation 队列任务
  → PersistentTaskQueue / RunningTask
  → PersistentTaskQueue 唯一生成 task id 并先登记内存投影
  → MemoryTaskQueue 仅按 task id 发放 permit
  → a4n-db queue::update_task_status 单事务
     ├─ task_queue 状态与时间戳
     └─ workflow_run.active_task_id

启动恢复
  → 中断非 Generation active task
  → recover_stale_in_process_workflows 唯一决定 Workflow interrupted/can_resume
  → 独立恢复 provider 前 stale video
```

- `MemoryTaskQueue` 只负责等待队列、并发槽位、取消令牌和内存投影。
- 同一 task id 的创建、持久化与调度入口由 `PersistentTaskQueue` 唯一拥有；不存在按类型领取任意任务的旁路。
- `a4n-db::queue::update_task_status` 是非 Generation queue 状态及 Workflow active-task 关联的唯一持久化入口。
- Generation lifecycle 仍由其专项事务独立拥有，不经过本入口重复写库。
- `video_provider_job` 保持跨重启 durable running；其 provider job 恢复由既有视频生命周期负责。

## 集中验证

- `cargo fmt --all -- --check`：通过。
- `cargo check -p a4n-queue -p a4n-db -p a4n-orchestrator -p a4n-runtime --all-targets`：通过，无 warning。
- `cargo check -p a4n-runtime --all-targets --features embedded-capabilities`：通过，覆盖 embedded LLM stream 编译路径。
- `cargo test -p a4n-queue --lib`：10 项通过，包含最小按 id 优先级调度回归测试。
- `cargo test -p a4n-db queue:: --lib -- --test-threads=1`：4 项通过。
- `cargo test -p a4n-orchestrator task_queue:: --lib --no-run`：通过。
- `timeout 60s cargo test -p a4n-orchestrator task_queue:: --lib -- --test-threads=1`：前 17 项通过后在最后两项 guard 测试处达到整组 60 秒上限；随后单独补跑这 2 项，均通过，因此该筛选范围 19 项全部通过。
- `cargo test -p a4n-orchestrator --test chapter_analysis_workflow workflow_input_uses_stable_business_key_and_content_fingerprint`：1 项通过。
- `git diff --check`：通过。

所有 Cargo 命令均使用仓库 `/home/fufu/Code/A4N/target/`。数据库测试先读取 `docs/TEST_DATABASE.md` 并加载仓库 `.env`，仅使用隔离 `a4n_test`。
