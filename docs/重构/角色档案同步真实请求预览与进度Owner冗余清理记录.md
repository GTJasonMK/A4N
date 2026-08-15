# 角色档案同步真实请求预览与进度 Owner 冗余清理记录

日期：2026-08-14

## 结论

角色档案同步请求预览已收敛到真实执行使用的唯一首请求构造链。单角色预览只返回当前持久化状态能够精确构造的 `character_sync_evidence_events` 请求；批量预览只为确需同步的角色返回各自首请求；已跳过角色返回零个请求。后续属性变化、章节状态、行为、删除候选、隐性分类和隐性复审均依赖真实模型输出，不再用空数组和 `blocked` 状态伪造。

角色同步进度也收敛为唯一 `CharacterProfileSyncProgress` 与唯一 `CHARACTER_PROFILE_SYNC_STEPS`。单角色、五阶段执行、批量同步和章节分析内嵌同步不再维护平行进度类型或常量；前端不再复制每角色 10 步的业务总数。

本批未修改命令名称、输入输出合同、权限、取消、Workflow、Checkpoint、事务写入、RAG 同步或真实五阶段执行顺序，未新增兜底、兼容、回退、占位或第二套通用预览抽象。

## BFS 搜索覆盖范围

### 第 0 层：用户功能入口

- `frontend/src/pages/CharacterDetailDialog.tsx`
- `frontend/src/pages/writing-desk/CharacterProfilesModal.tsx`
- `frontend/src/components/feedback/RequestPreviewModal.tsx`
- `frontend/src/hooks/useProgressTask.tsx`

覆盖单角色“从当前章节同步”、批量“同步全部角色档案”、角色发现、三类请求预览、任务取消、轮询进度、角色切换后的异步结果失效和空请求列表展示。

### 第 1 层：前端 API 与生成合同

- `frontend/src/api/character/state.ts`
- `frontend/src/api/character/profiles.ts`
- `frontend/src/api/character/types.ts`
- `frontend/src/api/generated/host.ts`
- `frontend/src/api/runtimeConfigSnapshot.ts`

确认执行与预览复用同一 `SyncCharacterProfileInput` / `SyncAllCharacterProfilesInput`，预览按 `without_secrets` 策略构造运行时配置，执行命令携带任务身份和完整 LLM 配置。

### 第 2 层：命令清单、Tauri、Web 与 CLI

- `crates/a4n-api-contract/src/character.rs`
- `crates/a4n-api-contract/src/command_manifest.rs`
- `crates/a4n-tauri/src/command_registry.rs`
- `crates/a4n-tauri/src/commands/dispatch.rs`
- `crates/a4n-server/src/routes/commands/dispatch/assets_and_media.rs`
- `crates/a4n-server/src/routes/commands/characters/character_ai.rs`
- `crates/a4n-cli/src/routing.rs`

确认 Tauri 只经统一 `invoke_host_command`，Web 只经 Manifest 与严格参数解析进入相同 Runtime，CLI 只经通用 `/commands/{command}` 路由；不存在桌面、Web 或 CLI 私有角色同步实现。

### 第 3 层：Runtime 权限、任务、预览与 Trace

- `crates/a4n-runtime/src/actions/character_ai/contracts.rs`
- `crates/a4n-runtime/src/actions/character_ai/scope.rs`
- `crates/a4n-runtime/src/actions/character_ai/progress.rs`
- `crates/a4n-runtime/src/actions/character_ai/preview.rs`
- `crates/a4n-runtime/src/actions/character_ai/sync_profile/execution.rs`
- `crates/a4n-runtime/src/actions/character_ai/sync_profile/preview.rs`
- `crates/a4n-runtime/src/actions/character_ai/sync_batch.rs`
- `crates/a4n-runtime/src/actions/character_ai/character_discovery_flow/request.rs`
- `crates/a4n-runtime/src/actions/character_ai/sync_trace.rs`
- `crates/a4n-runtime/src/actions/request_preview/status/assembly.rs`
- `crates/a4n-runtime/src/actions/request_preview/status/model.rs`

覆盖项目编辑权限、任务 scope、取消令牌、进度事件、RAG trace、LLM trace、请求预算、敏感字段清理和统一 `ModelRequestPreview` 输出。确认旧 `CharacterAiPreviewStep` 只在 Runtime 内进行一次无语义 DTO 转换后又转回通用预览步骤，没有独立合同价值。

### 第 4 层：Orchestrator 输入准备与范围门禁

- `crates/a4n-orchestrator/src/character/profile_sync/contracts.rs`
- `crates/a4n-orchestrator/src/character/profile_sync/context.rs`
- `crates/a4n-orchestrator/src/character/profile_sync/execution.rs`
- `crates/a4n-orchestrator/src/character/profile_sync_batch.rs`
- `crates/a4n-orchestrator/src/character/sync_rules.rs`
- `crates/a4n-orchestrator/src/context/project_chapter_context.rs`

覆盖按名称和已加载档案两种 subject、增量同步和章节分析重同步两种 mode、目标章节存在性、连续章节范围、首次出现章节、正文锚点、基线状态、已同步跳过和批量角色筛选。

### 第 5 层：真实请求构造与模型执行

- `crates/a4n-orchestrator/src/character/profile_sync_stages/contracts.rs`
- `crates/a4n-orchestrator/src/character/profile_sync_stages/request.rs`
- `crates/a4n-orchestrator/src/character/profile_sync_stages/execution.rs`
- `crates/a4n-orchestrator/src/character/profile_sync_stages/output.rs`
- `crates/a4n-orchestrator/src/character/implicit_classification/execution.rs`
- `crates/a4n-orchestrator/src/character/implicit_review/execution.rs`

真实执行顺序为：

    证据事件抽取
    → 属性变化判断（依赖真实证据事件）
    → 章节状态生成（依赖真实前序输出）
    → 关键行为记录（依赖真实证据事件）
    → 删除候选判断（依赖真实证据与属性变化）
    → 行为隐性属性分类（依赖真实 behavior_records）
    → 隐性属性复审（依赖同步结果与分类结果）

旧预览只对首阶段构造真实请求，却向 `previous_steps` 写入空数组来生成后续结构状态，并额外追加分类与复审 `blocked` 步骤。这些步骤不是执行路径会发送的请求，属于伪安全占位。

### 第 6 层：Workflow、Checkpoint、取消与恢复

- `crates/a4n-orchestrator/src/context/character.rs`
- `crates/a4n-orchestrator/src/context/workflow.rs`
- `crates/a4n-orchestrator/src/workflow/checkpoint.rs`
- `crates/a4n-orchestrator/src/task_queue/persistence/recovery/tests.rs`
- `crates/a4n-server/src/routes/commands/characters/character_ai.rs`

覆盖 `character_profile_sync` Workflow 指纹、阶段 call key、completed LLM checkpoint、恢复时 response/parsed output 一致性、active task、用户取消、失败/完成终态和批量部分失败。预览不创建 Workflow、不写 Checkpoint，真实执行语义未改变。

### 第 7 层：事务持久化、RAG 与 Prompt 配置

- `crates/a4n-db/src/character/sync_persistence/execution.rs`
- `crates/a4n-db/src/character/sync_persistence/write_steps.rs`
- `crates/a4n-db/src/character/sync_persistence/snapshots/*`
- `crates/a4n-orchestrator/src/rag/continuity_records/character_sync/execution.rs`
- `crates/a4n-runtime/src/actions/prompt_records.rs`
- `crates/a4n-prompt/src/rendering.rs`
- `prompts/_registry.yaml`
- `prompts/novel/07_protagonist/character_sync_*.md`
- `prompts/novel/07_protagonist/implicit_*.md`

覆盖事件、行为、删除候选、隐性复审、章节状态、快照、Generation run、RAG 删除与重建，以及 `PromptRenderer for AppState` 动态分派。提示词的唯一真实构造 owner 是 `build_character_profile_sync_stage_request`。

该路径未发现 XML DI、反射调用、动态代理或 AOP 切面。运行时动态分派仅来自显式 `PromptRenderer`、Trace port、进度 reporter 与 Host command table，均已纳入闭包。

## 优化记录

1. 角色同步预览只构造真实首阶段 `EvidenceEvents` 请求。
2. 删除预览为后续阶段写入空 `previous_steps` 数组的伪输出。
3. 删除属性变化请求 `blocked` 占位。
4. 删除章节状态请求 `blocked` 占位。
5. 删除行为记录请求 `blocked` 占位。
6. 删除删除候选请求 `blocked` 占位。
7. 删除行为隐性分类请求 `blocked` 占位。
8. 删除隐性属性复审请求 `blocked` 占位。
9. 已无需同步的单角色预览返回零个请求，不再伪造 `reused` 状态卡片。
10. 批量预览忽略已跳过角色，只保留确需同步角色的首请求。
11. `CharacterProfileSyncPreviewStep` 从可空 request 收紧为必有 request。
12. 删除该步骤可变 `status`，Runtime 唯一标记真实请求为 `ready`。
13. `note` 从可空收紧为真实同步范围文本。
14. stage 预览从 `Vec` 收紧为单个真实步骤。
15. 方法改名为 `prepare_character_profile_sync_initial_request_preview_validated`，删除“拥有全部阶段预览”的错误语义。
16. 防回归测试改为断言唯一首请求及真实 trace step key，不再断言七个占位步骤。
17. 删除 Runtime 重复中间类型 `CharacterAiPreviewStep`。
18. 删除其 `llm` 构造器。
19. 删除其 `status` 构造器。
20. 单角色预览不再先映射到私有 DTO 再映射回通用 DTO。
21. 删除批量预览私有 `preview_step` 双分支映射。
22. 删除只转发到 `chat_status_preview_for_state` 的 `character_ai_preview` facade。
23. 三类角色预览直接复用既有 `ChatStatusPreviewStep` 与统一预算/脱敏 owner。
24. 删除只被单一调用方使用的 `character_sync_preview_steps_for_input` 弹性层。
25. 单角色 Runtime helper 从复数 Vec 返回收紧为可空单步骤。
26. 角色发现预览改为直接使用通用 `ChatStatusPreviewStep`。
27. 删除角色发现预览无第二调用方的可变 `initial_state` 参数。
28. 删除角色发现预览无第二调用方的可变 `initial_note` 参数。
29. 将角色同步总步数收敛为顶层唯一 `CHARACTER_PROFILE_SYNC_STEPS`。
30. 删除 profile application 私有重复总数常量。
31. 删除 stage application 私有重复总数常量。
32. 删除 `CharacterProfileSyncStageProgressReporter` 重复别名。
33. 删除 `CharacterProfileSyncStageProgress` 重复结构和实现。
34. 五阶段执行直接复用 `CharacterProfileSyncProgress`，删除 reporter 重包装。
35. 删除 `CharacterProfileBatchProgressReporter` 重复别名。
36. 删除 `CharacterProfileBatchProgress` 重复结构和实现。
37. 批量同步与章节分析内嵌同步复用同一 `CharacterProfileSyncProgress`。
38. 删除 Runtime 对同步步数常量的无消费者公开重导出。
39. 删除 `sync_types` 内只做转发的常量重导出。
40. 新增唯一 mode-aware 跳过规则 `profile_sync_skip_reason_for_mode`。
41. 删除单角色执行内的重复 mode match。
42. 删除批量执行内的重复 mode match。
43. 批量进度文案删除硬编码除数 `10`，改用唯一常量。
44. 单角色前端任务删除硬编码总数 `10`，总数只由后端进度事件拥有。
45. 批量前端任务删除 `profiles.length * 10` 第二实现。
46. 六个角色同步 Prompt 的 `used_by` 从命令别名收敛为唯一真实请求构造函数。
47. 隐性分类与复审 Prompt 的 `used_by` 从入口命令别名收敛为各自唯一请求构造函数。
48. 角色发现前端任务删除硬编码总数 `100`，统一等待后端任务 scope 与进度事件。

## 清理后唯一事实标准

    单角色 / 批量预览
    → prepare_character_profile_sync_context
    → profile_sync_skip_reason_for_mode
       ├─ 需要跳过 → 0 个请求
       └─ 需要同步
          → prepare_character_profile_sync_initial_request_preview_validated
          → build_character_profile_sync_stage_request(EvidenceEvents, {})
          → ChatStatusPreviewStep(state = ready)
          → ModelRequestPreview

    单角色 / 批量 / 章节分析真实执行
    → 同一 prepare context 与 skip rule
    → 同一 CharacterProfileSyncProgress / CHARACTER_PROFILE_SYNC_STEPS
    → 五阶段真实模型输出链
    → 隐性分类与复审
    → 唯一事务持久化
    → RAG 同步
    → Workflow 终态

## 集中验证

所有 Cargo 命令复用 `/home/fufu/Code/A4N/target`：

- `cargo fmt --all`：通过。
- `cargo check -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri --all-targets`：通过。
- 真实首请求防回归测试：1/1 通过。
- mode-aware 跳过规则测试：1/1 通过。
- Prompt registry 专项：10/10 通过。
- `npm exec tsc -- --noEmit`：通过。
- 数据库测试前已读取 `docs/TEST_DATABASE.md`，并从仓库 `.env` 加载隔离的 `A4N_TEST_DATABASE_URL`；测试串行执行，未使用开发数据库。

## 保留边界

- 保留项目编辑权限、章节与锚点校验、输入范围校验、取消和显式失败。
- 保留真实五阶段 Prompt、输出校验、Checkpoint 指纹、截断响应拒绝和恢复一致性校验。
- 保留行为隐性分类与隐性复审；它们是执行中的真实业务阶段，只删除无法提前构造请求的预览占位。
- 保留事务化事件/状态/快照写入和 RAG 同步；它们不是重复实现。
- 保留通用请求预览对其他业务的 `blocked` 能力；本批只删除角色同步路径中不真实的消费者。
