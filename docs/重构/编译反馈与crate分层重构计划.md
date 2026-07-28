# 编译反馈与 crate 分层重构计划

## 目标

本计划的目标不是让 Rust 项目在修改后完全跳过编译，而是减少一次小改动牵动过多 crate 的概率，让日常开发能优先获得局部反馈，再按需要构建最终可执行程序。

具体目标：

1. 缩短日常 `cargo check` / `cargo build` 的反馈链路。
2. 降低底层 crate 的改动频率和扇出影响。
3. 把重依赖能力从轻量业务路径中拆开，避免修改 prompt、LLM provider、Tauri command 时被 RAG、embedding、vector DB 等依赖拖慢。
4. 保持现有业务行为不变，重构期间不引入第二套业务逻辑、权限逻辑、校验逻辑或 API 合同。
5. 让每一步都有独立验收命令和明确回退边界。

## 当前结论

截至 2026-07-15，本计划的 crate 拆分与 minimal host 主体已经落地，原始“编译放大点”不再代表当前代码：

1. `a4n-engine` 已退出 workspace，prompt、LLM、embedding、RAG、image、video、queue 等能力已经拆入独立合同/实现/provider/service crate。
2. migration 由 `a4n-migration` 独立管理，minimal runtime/server/Tauri 路径不再无条件编译迁移和 provider 重依赖。
3. `a4n-server` / `a4n-tauri` 通过 `a4n-runtime::host_api` 与 orchestrator/client/protocol 边界访问业务；minimal host 依赖树已确认不含 LanceDB、Tantivy、FastEmbed、HF Hub、Git2、图片/视频 provider 和 migration 工具链。
4. prompt、LLM、embedding、RAG、image、video、import、export 八个 sidecar binary 已具备统一 v2 协议、capability registry、进程管理和 package inventory；sidecar 地址、日志、URL、profile 与 provider 配置错误均显式失败。
5. 已删除本轮反向审计发现的恒定成功 hook、RAG `create_collection` 假成功 API、无消费者公共别名/DB helper、旧环境变量别名、模型发现 v1 漂移和 embedded fallback；sidecar 请求/响应均严格拒绝未知字段。
6. Linux 当前环境已完成 `ort-sys`/FastEmbed/Lance 等原生重依赖编译，并实际验证 prompt、LLM、embedding、RAG、image、video、import、export 八项 sidecar 均为 process=running、health=healthy、protocol=2、v2 capability。主要剩余工作转为真实 PostgreSQL、sidecar HTTP 故障注入、跨重启恢复，以及 Windows/macOS/Linux 签名安装器和发布包实跑。
7. 第二轮反向审计继续删除零生产消费者入口、严格化 runtime config 单一线格式，并清理工作流/RAG/视频/导出/上下文预算等数值边界中的饱和运算、截断转换和假进度回退；生产源码已无 `serde(alias)`、`saturating_add`、`saturating_mul`、`as u32`、整数上限兜底或 `allow(dead_code)` 残留。

日常构建和模块级反馈路径已经固化：

1. `scripts/build.sh` 支持 `-p/--package` 与 `--check`，可只检查指定 package 及其依赖。
2. `scripts/dev/check-module.sh`、`run-host.sh`、`run-services.sh`、`restart-service.sh` 和 `service-status.sh` 提供模块级检查、启动与重启入口。
3. `host-server` / `host-tauri` minimal 检查、sidecar package inventory、前端 API parity 和 workspace metadata 已纳入收尾门禁。
4. full/embedded profile 保留为交付前兜底，但不再是日常 host 默认编译路径。

## 原始代码复核补充（历史基线）

本节及其后的阶段性依赖图、旧路径示例和 `a4n-engine` 验收命令记录的是重构启动时的历史基线，用于解释拆分决策，不再代表 2026-07-15 的当前代码或可执行命令。当前状态以文首“当前结论”、文末“完成定义”和实际 workspace metadata 为准；后续不得再执行 `cargo check -p a4n-engine` 或恢复任何 `a4n_engine::*` 兼容路径。

结合当时代码复核后，原计划补充了以下约束。

### 1. `a4n-engine` 不只是 AI engine

当前 `crates/a4n-engine/src/lib.rs` 暴露：

```rust
pub mod embedding;
pub mod image;
pub mod llm;
pub mod prompt;
pub mod queue;
pub mod rag;
pub mod video;
```

其中 `queue` 已被 `a4n-services` 和 `a4n-runtime` 作为公共任务队列合同使用，例如 `a4n_services::queue::PersistentTaskQueue` 依赖 `a4n_engine::queue::memory_queue::MemoryTaskQueue` 和 `a4n_engine::queue::priority::*`。

因此后续拆分不能只按 prompt/LLM/RAG 理解 `a4n-engine`。需要新增一个队列边界决策：

1. 如果任务队列类型是跨 runtime/services 的公共合同，优先新建 `a4n-queue`；只有已经存在且足够稳定的事件类型才暂留 `a4n-core`。
2. 如果只是运行时实现，应迁到 `a4n-runtime` 或 `a4n-services::queue`。
3. 在这个决策完成前，不能把 `a4n-engine` 视为可以快速拆空的 crate。

### 2. prompt 模块当前绑定 Git 持久化

`crates/a4n-engine/src/prompt/git_store.rs` 直接依赖 `git2`，`TeraPromptEngine::with_git_store` 又在 `a4n-runtime/src/state.rs` 的 `create_prompt_engine` 中默认启用。

因此拆 `a4n-prompt` 时不能假设它一定是轻量 crate。需要二选一：

1. 接受 `a4n-prompt` 依赖 `git2`，优先完成模块边界拆分。
2. 把 Git 持久化拆成 `a4n-prompt-git` 或 `a4n-prompt` 的 `git-store` optional feature。

推荐第一步先接受 `git2` 留在 `a4n-prompt`，因为这能避免同时改 prompt 持久化行为。第二步再评估是否 feature 化 Git 存储。

### 3. RAG 当前在启动时无条件初始化

`AppState::new` 当前总是执行：

```rust
LanceVectorStore::open(config.data_dir.join("lance")).await?
TantivyTextIndex::new(config.data_dir.join("tantivy"))?
HybridVectorStore::new(lance_vec, tantivy_idx)
RagService::new(vector_store)
```

这意味着即使把 RAG 代码拆到 `a4n-rag`，只要 `AppState` 默认初始化 RAG，桌面端和服务端仍会编译并初始化 RAG 相关能力。

因此 `a4n-rag` 拆分前要先决定运行时策略：

1. full profile：启动时初始化 RAG，保持当前行为。
2. minimal profile：不初始化 RAG；调用 RAG action 时显式返回能力未启用错误。
3. lazy profile：首次 RAG 调用时初始化 LanceDB/Tantivy，失败显式返回。

不建议一开始做 lazy profile，因为状态、并发和错误边界更复杂。更稳的是先做 full/minimal 两个清晰 profile。

### 4. `a4n-core` 当前混合了 SeaORM entity 和 API/领域类型

`a4n-core/src/models/**` 大量文件使用 `sea_orm::entity::prelude::*` 和 `DeriveEntityModel`。因此 `a4n-core` 不是纯合同 crate，而是“数据库实体 + 共享类型 + 配置 + 事件”的组合。

这会影响长期的 `a4n-contracts` / `a4n-domain` 设想：

1. 短期不要直接把 `a4n-core` 拆成多个 crate。
2. 如果要拆 `a4n-contracts`，必须先区分 SeaORM entity、API DTO、前端合同、领域值对象。
3. 在这个区分完成前，`a4n-core` 的优化策略应是“避免继续膨胀”，不是立刻迁移既有模型。

### 5. image/video 也应纳入 engine 拆分序列

当前 image/video provider 位于 `a4n-engine`：

```text
crates/a4n-engine/src/image/
crates/a4n-engine/src/video/
```

并被 runtime/services 直接使用：

```text
a4n_services::image::provider_config -> a4n_engine::image::{ImageProvider, pick_image_provider}
a4n_services::video::config          -> a4n_engine::video::{VideoProvider, pick_video_provider}
a4n_runtime::actions::image_generation -> ImageGenerationRequest / ImageReference
a4n_runtime::actions::video_generation -> VideoGenerationRequest / provider preview helpers
```

因此长期目标里 `a4n-media` 太粗，需要拆成至少两个方向：

1. `a4n-image`：图片生成 provider、请求/响应模型、preview helper。
2. `a4n-video-provider` 或 `a4n-video-engine`：视频 provider、轮询、请求预览、provider-specific request body。

不过这两块当前和业务动作耦合较深，不应早于 prompt/LLM/queue 边界处理。

### 6. Cargo feature 只能做能力裁剪，不能替代依赖拆分

当前 workspace 没有内部 feature profiles。后续增加 feature 时要注意 Cargo features 是 additive 的：同一个构建图里只要某个路径启用了 feature，依赖会按启用后的并集编译。

因此：

1. feature 适合区分 `minimal` / `full` 入口。
2. 如果常用入口仍默认启用所有能力，feature 不会改善日常编译。
3. 真正降低编译牵连仍要靠 crate 边界拆分和入口依赖收敛。

## 二次代码复核补充

进一步结合 runtime/server/action 代码后，完整微服务化还必须处理以下实际约束。

### 1. `AppState` 是服务定位器，不能直接带进 sidecar 时代

当前 `AppState` 同时持有数据库、配置、事件、任务、所有业务服务和 provider：

```rust
pub db: Arc<DatabaseConnection>
pub config: AppConfig
pub event_bus: Arc<CoreEventBus>
pub task_manager: Arc<TaskManager>
pub rag: RagService
pub task_queue: Arc<PersistentTaskQueue>
pub llm: Arc<dyn LlmProvider>
pub engine: Arc<dyn PromptEngine>
pub embedding: Arc<dyn EmbeddingProvider>
```

大量 action 直接接收 `&AppState` 并读取 `state.db`、`state.llm`、`state.engine`、`state.embedding`、`state.config.data_dir`。`a4n-server::ServerState` 还通过 `Deref<Target = a4n_runtime::AppState>` 让 route 直接访问 runtime。

因此 sidecar 化前需要一个明确过渡层：

1. 新建 `A4nCapabilities` 或 `ServiceClients`，集中持有 `PromptClient`、`LlmClient`、`EmbeddingClient`、`RagClient`、`ImageClient`、`VideoClient`。
2. `AppState` 保留数据库、权限、任务、配置和业务服务，但 provider 访问改走 capabilities。
3. action 逐步从 `state.llm.as_ref()` 改成 `state.capabilities.llm()`。
4. `ServerState` 不应长期 `Deref` 到 `AppState`；Web 层应显式调用 runtime/orchestrator facade。

否则 sidecar 方案会变成到处修改调用点，风险远高于先做 facade。

### 2. `runtime_config` 是 task-local，跨进程会丢失

当前运行时配置通过 `tokio::task_local!` 保存：

```rust
RUNTIME_CONFIG_SNAPSHOT
RUNTIME_USER_ID
```

`DynamicLlmProvider`、图片/视频配置解析、canvas workflow 等逻辑会读取 `runtime_config::current_snapshot()` 和 `current_user_id()`。sidecar 是独立进程，不会继承 task-local。

因此 sidecar 请求必须显式携带：

```text
runtime_config_snapshot
runtime_user_id
redacted_runtime_config_snapshot
secret material policy
```

尤其是系统默认模板和用户 API key：

1. host 不能把未授权用户的系统密钥传给 sidecar。
2. remote sidecar 不应接收本地解密后的长期 secret，除非该部署明确受信。
3. runtime snapshot 的 schema version 要纳入 sidecar 协议版本协商。

### 3. 任务进度和取消当前是进程内机制

当前任务取消依赖 `tokio_util::sync::CancellationToken`，进度依赖进程内 `TaskManager` / `TaskHandle` / `WebEventHub` / Tauri event bridge：

```text
state.task_manager.cancel(task_id)
TaskHandle.cancel_token
TaskHandle.update_progress(...)
WebEventHub.publish("workflow:{task_id}", ...)
Tauri bridge emit_workflow_event(...)
```

sidecar 中不能直接共享这些内存对象。必须新增跨进程任务协议：

```text
POST /tasks/{task_id}/cancel
GET  /tasks/{task_id}
GET  /tasks/{task_id}/events
sidecar -> host progress event callback 或 host 轮询
```

推荐由 orchestrator 拥有 task id 和用户可见状态，sidecar 只上报 `ServiceTaskEvent`：

```text
started
progress
chunk
artifact
completed
failed
cancel_ack
heartbeat
```

host 收到 sidecar event 后再映射到现有 SSE/Tauri 事件，避免前端事件合同被每个 sidecar 复制。

### 4. 当前存在两套 TaskType，需要收敛

目前至少有两套任务类型：

```text
a4n_core::event::channel::TaskType
a4n_engine::queue::priority::TaskType
```

它们枚举值并不完全一致。sidecar 化后如果继续扩散，会导致任务状态、进度阶段和队列类型之间出现映射漂移。

因此 `a4n-queue` 或 `a4n-service-protocol` 阶段必须定义一个 canonical task kind，并提供旧类型到新类型的显式映射。不能让每个 route/action 自己转字符串。

### 5. 媒体和临时文件现在强依赖本地 `data_dir`

当前媒体读取、上传和清理大量依赖本地路径：

```text
state.config.data_dir
canonicalize_path_in_data_dir
relative_media_path
temp-media/video-inputs
media_file::read_media_file_response
```

这对 embedded 模式没问题，但 remote/sidecar 模式下不能假设 sidecar 和 host 共享文件系统。

必须新增 artifact store 协议：

```text
artifact_id
artifact_kind
content_type
size_bytes
sha256
storage_uri
expires_at
ownership: host | sidecar | remote
```

规则：

1. 本地 sidecar 可以使用共享 data/cache/tmp 目录，但路径必须由 host 授权并限制在允许目录内。
2. remote sidecar 只能通过上传 URL、下载 URL 或 artifact API 交换文件。
3. sidecar 返回的文件不能直接进入数据库，必须由 orchestrator 校验、复制到 data_dir、落库。

### 6. schema 迁移不只在 `a4n-migration`

`AppState::new` 除了运行 `a4n_migration::migrator::run_migrations`，还包含 `ensure_compatible_schema`，里面有多条 `ALTER TABLE` 和 `CREATE INDEX`。

这意味着 schema 规则目前有第二来源。微服务化前必须收敛：

1. 所有 schema 变化进入 `a4n-migration`。
2. `ensure_compatible_schema` 只能作为短期兼容检查，不继续添加新 schema 变更。
3. sidecar 不得各自执行主库 schema 迁移。
4. service protocol 的版本必须能声明所需 schema version。

否则迁移工具独立化后仍会有 runtime 偷偷改 schema，破坏可部署性。

### 7. LLM context pack 当前在 services 层直接写库并依赖 ChatRequest 类型

`a4n-services/src/llm_context.rs` 同时依赖：

```text
a4n_core::models::generation::llm_context_pack
a4n_engine::llm::provider::{ChatMessage, ChatRequest, MessageRole}
sea_orm
```

这意味着 LLM context 不只是纯 prompt/LLM 逻辑，它包含预算规划、请求重写、context pack 落库、ChatRequest 类型。拆 LLM gateway 时必须保持清晰边界：

1. context pack 落库由 orchestrator 做，gateway 只接收渲染后的 messages。
2. gateway 只能返回 usage、finish_reason、trace metadata 和 provider diagnostics，不默认写 `llm_context_pack` / `llm_call_trace`。
3. 如果未来 gateway 被提升为唯一 trace owner，必须作为单独架构决策处理，不能在本阶段顺手引入。

推荐先保持 orchestrator 落库，LLM gateway 只负责 provider 调用、流式、usage、模型能力和 provider 错误规范化。

## 最终代码审查补充

本节是最后一次结合当前代码做的补充审查。结论是：原方案主线正确，但如果目标是“能拆的都拆，尽量做到改哪个模块就只编译哪个模块并替换使用”，还必须把以下横向能力显式纳入计划。

需要先承认一个边界：当前后端 Rust 文件约 833 个，且包含大量数据库状态机、前端命令映射、文件包校验、流式事件和运行时配置逻辑。静态代码审查可以覆盖主要编译放大点和架构边界，但不能负责任地宣称“已经 100% 覆盖所有代码所有功能的运行时行为”。因此完整计划必须把后续每阶段的依赖审计、API parity、目标功能 smoke test 作为完成条件，而不是只靠一次文档判断。

### 1. 入口 crate 对 engine 的直接依赖必须单独清理

`cargo tree -p a4n-tauri --depth 2` 和 `cargo tree -p a4n-server --depth 2` 显示，两个入口都直接依赖：

```text
a4n-engine
a4n-services
a4n-migration
a4n-runtime
```

其中 `a4n-engine` 又无条件带入：

```text
fastembed
hf-hub
lancedb
tantivy
git2
arrow-array
arrow-schema
image
reqwest
```

这意味着即使 `a4n-runtime` 内部改成 capability facade，只要 `a4n-tauri` / `a4n-server` 的 `Cargo.toml` 仍直接依赖 `a4n-engine`，host 就不会真正瘦身。

必须新增硬性验收：

```bash
cargo tree -p a4n-tauri --depth 2 | rg "a4n-engine|lancedb|fastembed|tantivy|git2"
cargo tree -p a4n-server --depth 2 | rg "a4n-engine|lancedb|fastembed|tantivy|git2"
```

在 host minimal profile 完成后，上述命令不应命中重 provider 实现；如果还命中，说明 sidecar 化只是形式上完成，编译目标没有达成。

### 2. 前端 API 合同不能被 sidecar 拆散

当前前端统一通过 `frontend/src/api/index.ts` 的 `apiInvoke` 访问后端：

```text
Tauri runtime -> @tauri-apps/api/core invoke(cmd, args)
Web runtime   -> POST /commands/{cmd}
```

项目已有 `scripts/checks/check-web-api-parity.mjs` 和 `cd frontend && npm run check:web-api-parity`，用于校验前端命令、Tauri command、Web command route 的一致性。

因此 sidecar 不能直接暴露给前端，也不能让某个业务页面绕过 host 调 sidecar。正确边界是：

```text
frontend
  -> apiInvoke / SSE / media URL
  -> a4n-tauri 或 a4n-server host
  -> orchestrator / capability client
  -> sidecar
```

新增验收要求：

1. 每次新增、移动或删除业务 command，都必须跑 `cd frontend && npm run check:web-api-parity`。
2. sidecar API 只作为 host 内部协议，不进入 `frontend/src/api/**`。
3. 前端继续只监听 host 发出的 `workflow:{task_id}` 事件，不直接监听 sidecar 事件。
4. 如果未来生成 `a4n-api-contract` / TypeScript 类型，生成源必须是 host/orchestrator API 合同，而不是某个 sidecar 的私有协议。

### 3. runtime config 的源头在前端和 host 双侧，不能只看 Rust task-local

二次复核已指出 `runtime_config` 是 task-local。最终审查进一步确认：配置快照源头还包括前端：

```text
frontend/src/api/runtimeConfigSnapshot.ts
frontend/src/api/localRuntimeSettings.ts
crates/a4n-runtime/src/runtime_config.rs
crates/a4n-runtime/src/actions/config_records.rs
crates/a4n-runtime/src/dynamic_llm.rs
crates/a4n-runtime/src/dynamic_embedding.rs
```

`runtimeConfigSnapshot.ts` 会根据命令名判断是否携带 `runtime_config_snapshot`，并决定是否包含 API key。服务化后不能只把 Rust task-local 改成请求字段，还必须统一以下策略：

1. 哪些命令需要 runtime snapshot。
2. 哪些命令允许携带用户或系统 API key。
3. preview 命令默认不带密钥的语义要保留。
4. remote sidecar 是否允许接收解密后的长期 secret。
5. sidecar error 中不能回显 base_url、api_key、JWT、上传 token 等敏感字段。

推荐新增 `SecretForwardingPolicy`，可以放入 `a4n-service-protocol`：

```text
SecretForwardingPolicy
  deny
  allow_local_sidecar_only
  allow_remote_trusted

RuntimeConfigSnapshotEnvelope
  schema_version
  user_id
  client_context
  redacted_snapshot
  secret_material_scope
```

否则 sidecar 拆分会引入安全边界模糊，尤其是 Web server 模式和本地 Tauri 模式混用时。

### 4. 流式输出不仅是 progress event，还包括 chunk 顺序、usage 和 checkpoint

当前流式链路至少涉及：

```text
crates/a4n-services/src/structured_llm_stream.rs
crates/a4n-services/src/structured_progress.rs
crates/a4n-core/src/event/sse.rs
crates/a4n-core/src/event/channel.rs
crates/a4n-tauri/src/events/bridge.rs
crates/a4n-tauri/src/progress_task.rs
crates/a4n-server/src/state.rs WebEventHub
crates/a4n-services/src/llm_checkpoint.rs
crates/a4n-services/src/workflow.rs
```

因此 `ServiceTaskEvent` 不能只定义 `progress/completed/failed`。它还要覆盖：

```text
chunk_index
chunk_delta
structured_progress_event
usage_delta / final_usage
trace_context
checkpoint_key
finish_reason
cancel_ack
heartbeat
```

LLM sidecar 尤其要保留 `usage`、`finish_reason`、`is_truncated_finish_reason` 和 checkpoint 语义。否则章节生成、导入反推、优化 agent、视频提示词生成这类流程会出现“服务能返回文本，但 host 无法正确续跑、统计、取消或判定截断”的问题。

### 5. workflow/checkpoint/generation_run 是编排账本，不能下放给任意 sidecar

当前 workflow 相关逻辑分布在：

```text
crates/a4n-services/src/workflow.rs
crates/a4n-services/src/llm_checkpoint.rs
crates/a4n-services/src/generation/mod.rs
crates/a4n-runtime/src/actions/workflow_checkpoint.rs
crates/a4n-runtime/src/actions/task_records.rs
crates/a4n-runtime/src/actions/generation_analytics.rs
```

sidecar 可以执行 provider 调用和重计算，但不应各自写 `workflow_run`、`workflow_checkpoint`、`generation_run`、`llm_call_trace`。这些表承担幂等、恢复、统计和审计职责，应该由 orchestrator 统一写入。

允许的例外只有一种：如果未来某个 workflow executor 本身成为独立服务，那么它必须成为唯一 workflow owner，并通过同一套 repository/transaction 合同写库，不能多个服务同时写同一套 workflow 状态。

### 6. 导入、导出、项目包、全配置备份不能按普通 provider sidecar 处理

当前相关代码包括：

```text
crates/a4n-runtime/src/actions/project_transfer.rs
crates/a4n-runtime/src/actions/project_transfer/family_package.rs
crates/a4n-runtime/src/actions/project_transfer/video_studio_package.rs
crates/a4n-runtime/src/actions/settings_all/*
crates/a4n-runtime/src/actions/export_files.rs
crates/a4n-services/src/export/*
crates/a4n-services/src/import/*
```

这些功能看起来像“文件转换”，但实际包含：

```text
项目权限校验
事务写库
ID remap
状态机校验
hash 校验
媒体文件路径校验
source snapshot 校验
导入后 postprocess / RAG 同步
配置 secret 字段导入导出语义
```

因此拆分原则是：

1. 纯解析、纯渲染、纯压缩包读写可以拆为 `a4n-import-export-core`，用于局部编译。
2. 读取/写入数据库、权限校验、ID remap、项目状态推进必须留在 orchestrator。
3. 如果做 import/export sidecar，只能处理无状态文件加工，返回 manifest 和 artifact，不直接写业务库。
4. settings all 导入导出涉及 `api_key_encrypted`，必须跟 secret policy 统一处理。

这类能力可以优化编译边界，但不是第一批达成“替换服务即可用”的最佳对象。

### 7. 媒体路径、临时上传、公网 URL 和 token 是 artifact 协议的一部分

二次复核已指出本地 `data_dir` 假设。最终审查进一步确认，Web server 还有临时媒体和公开 URL 逻辑：

```text
crates/a4n-server/src/routes/temp_media.rs
crates/a4n-server/src/routes/media.rs
crates/a4n-runtime/src/actions/video_generation/media.rs
crates/a4n-runtime/src/media_file.rs
crates/a4n-runtime/src/file_utils.rs
frontend/src/api/runtime.ts toRuntimeMediaSrc()
```

视频 provider 可能需要公网可访问的输入图片/视频 URL，本地 Tauri 则常用本地路径或 `a4n-media:` protocol。sidecar/remote 模式必须在 artifact 协议里包含：

```text
artifact_id
kind
content_type
size
sha256
local_path
download_url
upload_url
expires_at
auth_mode
owner_user_id
project_id
```

并明确谁负责生成短期 token、谁负责清理临时文件、sidecar 是否允许回传外部 URL。否则 image/video/RAG sidecar 在桌面和 Web 两种部署下会出现行为分叉。

### 8. 配置测试、模型发现、本地 embedding 下载是独立能力边界

配置相关 action 不只是 CRUD：

```text
crates/a4n-runtime/src/actions/config_records.rs
crates/a4n-runtime/src/actions/embedding_model.rs
crates/a4n-tauri/src/commands/config/local_embedding.rs
frontend/src/api/config/embedding.ts
```

这里包含：

```text
test_llm_config / test_image_config / test_video_config / test_embedding_config
detect_*_config_models
local_* embedding model list/download/delete/embed_texts
```

这部分和 provider 能力强相关，也会拉入 `fastembed`、HTTP client、provider request preview 等重依赖。拆分时不能只迁移“生成”路径，还要迁移“配置测试/模型发现/本地模型管理”路径。

推荐归属：

1. provider probe 进入对应 `llm/image/video/embedding` client 或 sidecar。
2. config CRUD 仍在 orchestrator，因为它写库并受权限/管理员策略约束。
3. 本地 embedding 模型下载在桌面端可以由 host 调 local sidecar；Web server 默认不执行用户本地下载。
4. 相关前端命令继续由 host 映射，不能让页面直接调用 sidecar。

### 9. prompt sidecar 不能只提供 render

`AppState` 里有 `CONTRACT_BOUND_PROMPT_PATHS`、`RETIRED_BACKEND_PROMPT_NAMES`，prompt action 还包括：

```text
prompt_list
prompt_load_defaults
prompt_update
prompt_reset
prompt_version_history
prompt_version_diff
prompt_validate_variables
prompt_dependency_graph
prompt_render_ab
prompt project overrides
```

因此 `a4n-prompt-service` 至少要覆盖：

```text
render
list/registry
validate variables
dependency graph
version history/diff
default loading
project override lookup
AB render
```

或者明确其中哪些仍由 host 本地执行。不能只把 `PromptEngine::render` 做成 RPC，就声称 prompt 模块已完整 sidecar 化。

### 10. 测试工具和 mock provider 也要拆出稳定位置

当前大量测试直接使用：

```text
a4n_engine::prompt::tera_engine::TeraPromptEngine
a4n_engine::llm::provider::{LlmProvider, ChatRequest, ChatResponse}
a4n_engine::embedding::provider::EmbeddingProvider
```

如果拆 crate 时只迁移生产代码，不整理测试工具，会出现两个问题：

1. 上层 crate 的测试为了 mock provider 继续依赖旧 `a4n-engine`，导致重依赖仍进入测试编译。
2. mock 类型散落在业务测试里，后续 sidecar client 合同变更时测试维护成本上升。

推荐新增：

```text
a4n-test-support
  mock_llm
  mock_embedding
  in_memory_prompt_engine
  temp_app_state_builder
  service_client_fakes
```

并保证它只作为 dev-dependency 使用，不进入 release host 依赖树。

### 11. “微服务式”目标应定义为 host + sidecar，不是多个服务共享随意写库

从当前代码看，项目有强一致业务状态和大量事务边界，不适合直接拆成传统微服务各自拥有数据库。更准确的目标应写成：

```text
模块化 monorepo
  + 瘦 host/orchestrator
  + 多个可独立编译/重启/替换的 sidecar capability service
  + 单一数据库写入和权限事务边界
```

真正可以“改了哪里就编译哪里并替换使用”的主要是 provider/capability sidecar：

```text
prompt service
LLM gateway
embedding service
RAG service
image service
video service
可选 import/export pure transform service
```

不适合第一阶段服务化为独立写库微服务的能力：

```text
权限
项目状态机
source sync 决策落库
workflow/checkpoint/generation_run
project transfer import 写库
settings all import 写库
media ownership/binding 写库
前端 command 合同
```

这些应通过 crate 分层先实现局部编译收益，再在 orchestrator 内部收敛，不应被拆到多个进程各自维护业务真源。

## 设计冲突审查与统一口径

本节用于消除文档内部可能产生执行歧义的设计冲突。后续实施时以这里的统一口径为准。

### 1. LLM gateway 与 trace 写库边界

前文有两种表述容易冲突：

1. LLM gateway 负责 `trace/stream/usage`。
2. sidecar 不应写 `llm_call_trace`，trace 由 orchestrator 统一落库。

统一口径：

```text
LLM gateway 负责 provider 调用、stream、usage 解析、finish_reason 规范化、trace metadata 生成；
orchestrator 负责 llm_call_trace、llm_context_pack、generation_run、workflow_checkpoint 的落库。
```

也就是说，LLM gateway 可以返回 trace metadata / usage / provider diagnostics，但默认不直接写主业务数据库。只有当未来 LLM gateway 被明确升级为唯一 trace owner，并共享同一套迁移、权限上下文和 repository 合同时，才允许它直接写 trace 表；这不是本计划默认路径。

### 2. import/export service 与业务写库边界

前文曾把 `a4n-import-service` 写成 `import parsing / reverse / analysis`，这会和“import/export sidecar 只能做无状态文件加工”冲突。

统一口径：

```text
a4n-import-service 只做文件解析、格式识别、纯校验、包读取、manifest 生成；
a4n-export-service 只做纯渲染、压缩包组装、格式转换；
反推、AI 分析、项目写入、ID remap、权限校验、postprocess 仍由 orchestrator 执行。
```

因此 import/export service 是可选的 pure transform sidecar，不是业务写库微服务。

### 3. workflow-service 与 orchestrator 的唯一 owner 关系

前文列出 `a4n-workflow-service`，但又要求 workflow/checkpoint/generation_run 由 orchestrator 统一控制。

统一口径：

1. 默认方案没有独立写库的 `a4n-workflow-service`，workflow owner 是 `a4n-orchestrator`。
2. 如果未来确实需要独立 workflow executor，它必须成为唯一 workflow owner，不能与 orchestrator 同时写同一套 workflow 表。
3. 在本计划阶段 0-16 中，workflow/checkpoint/generation-run 的持久化合同由 `a4n-db::workflow` 持有，`a4n-orchestrator` 是默认执行者；在没有第二个真实消费者前不保留空的 `a4n-workflow-core` 占位 crate。

### 4. `a4n-core`、`a4n-contracts`、`a4n-api-contract` 的关系

文档中既说 `a4n-core` 可以放稳定 DTO/API contract，又规划了 `a4n-contracts` 和 `a4n-api-contract`。统一口径：

```text
短期：既有稳定共享类型可以继续留在 a4n-core，避免大范围迁移。
中期：新增跨进程 RPC 合同优先进入 a4n-service-protocol / a4n-contracts。
中期：新增前端/Tauri/Web API 合同优先进入 a4n-api-contract。
长期：a4n-core 逐步收敛为当前 SeaORM entity + 少量基础共享类型，不继续膨胀成合同垃圾桶。
```

### 5. embedded、fallback、minimal/full profile 的关系

文档同时提到保留 embedded 作为 fallback，以及 sidecar 不可用时不能静默 fallback。统一口径：

1. `embedded` 是显式配置的运行模式，不是自动 fallback。
2. `fallback` 只能是显式配置，例如 `allow_embedded_fallback = true`，并且日志和服务状态必须可见。
3. `minimal` profile 的目标是 host 不编译重 provider 实现。
4. `full` profile 用于 CI、回归和交付兜底，可以编译全部能力，但不作为日常快速开发默认路径。

### 6. 过渡 crate 名称与最终 crate 名称

文档中存在 `a4n-rag`、`a4n-video-provider`、`a4n-media` 等过渡名称，也存在 `a4n-rag-core`、`a4n-rag-index`、`a4n-video`、`a4n-video-providers` 等最终名称。

统一口径：

1. 阶段 4-8 可以使用较粗的过渡 crate 名称，降低第一次迁移成本。
2. 完整目标方案以 `最终 workspace 划分` 为准。
3. `a4n-media` 只能作为通用媒体合同/工具 crate，不能重新吸收 image/video provider。

## 完整目标方案：微服务式模块化

如果目标是“改了哪个模块就只编译哪个模块，编译完成后直接替换并可用”，单纯 Rust crate 拆分不够。Rust 默认是静态链接：库 crate 的代码最终会链接进 `a4n-tauri` 或 `a4n-server`，库 crate 改动后最终可执行程序仍要重新链接。

要接近微服务架构，需要把能力边界分成三类：

| 边界类型 | 是否只编译变更模块 | 是否能直接替换运行中能力 | 适用范围 |
| --- | --- | --- | --- |
| crate 内部模块 | 可以局部 `cargo check`，最终入口仍需重新链接 | 不可以 | 轻量纯逻辑、稳定库 |
| 独立 binary/sidecar 服务 | 可以，只编译该服务 | 可以，重启该服务即可 | RAG、LLM gateway、image/video provider、embedding、prompt registry |
| 动态插件 `.so/.dll` / WASM | 可以 | 可以，但 ABI/安全边界复杂 | 少量需要热插拔的算法或 provider |

本项目推荐采用 **monorepo + sidecar 微服务式架构**，而不是 Rust dylib 插件优先。原因：

1. Rust 没有稳定 Rust ABI，跨 `.so/.dll` 直接传 trait/object 风险高。
2. sidecar 通过 HTTP/gRPC/JSON-RPC 等协议通信，边界清晰，可单独编译、部署、重启。
3. Tauri 桌面端和 Web server 都可以共享同一组 sidecar 服务。
4. 重依赖能力最适合做 sidecar：RAG、embedding、本地模型、图片/视频 provider、prompt registry。

### 最终运行形态

长期目标运行形态：

```text
A4N Desktop / Web Host
  ├─ a4n-tauri              # 桌面壳，只负责窗口、协议适配、本地服务管理
  ├─ a4n-server             # Web API host，只负责 HTTP/API 适配
  ├─ a4n-orchestrator       # 业务编排 host，加载配置、管理任务、调用模块服务
  │
  ├─ a4n-prompt-service     # prompt registry / render / version / overrides
  ├─ a4n-llm-gateway        # LLM provider / streaming / usage / trace metadata
  ├─ a4n-embedding-service  # embedding provider / model download / model identity
  ├─ a4n-rag-service        # RAG ingest / search / vector DB / Tantivy / LanceDB
  ├─ a4n-image-service      # image generation providers / request preview
  ├─ a4n-video-service      # video generation providers / poll / request preview
  ├─ a4n-workflow-service   # 可选：仅当它成为唯一 workflow owner 时启用
  ├─ a4n-import-service     # 可选：纯解析/包读取/manifest，不做反推分析和写库
  ├─ a4n-export-service     # 可选：纯渲染/压缩包组装，不做权限和写库
  └─ a4n-migration          # 独立迁移工具，不作为默认 runtime 依赖
```

开发时：

```text
改 prompt 服务    -> cargo build -p a4n-prompt-service    -> 重启 prompt sidecar
改 RAG 服务       -> cargo build -p a4n-rag-service       -> 重启 rag sidecar
改 image provider -> cargo build -p a4n-image-service     -> 重启 image sidecar
改 Tauri command  -> cargo build -p a4n-tauri             -> 重启桌面壳
改纯合同 crate    -> 受影响服务都要重新编译，这是合理的全链路合同变更
```

### 最终 workspace 划分

长期 workspace 可拆为：

```text
crates/
  a4n-contracts             # 跨进程稳定合同：DTO、错误码、事件、OpenAPI/schema source
  a4n-core                  # 当前 SeaORM entity、配置、事件；逐步收敛，不立刻大拆
  a4n-domain                # 纯领域规则和值对象，不依赖数据库和 provider
  a4n-db                    # 数据库连接、repository trait/实现、事务边界
  a4n-api-contract          # Tauri/Web API 入参出参合同、生成 TypeScript 类型
  a4n-service-protocol      # sidecar RPC 协议、健康检查、版本协商
  a4n-capabilities          # host 内部能力 facade/client trait
  a4n-service-manager       # sidecar 进程管理、服务发现、健康检查

  a4n-queue                 # 队列合同、内存队列、任务状态类型
  a4n-runtime               # 本地 monolith runtime 组装，逐步瘦身为 orchestrator 客户端

  a4n-prompt                # prompt render 核心库
  a4n-prompt-git            # prompt Git 版本存储
  a4n-prompt-service        # prompt sidecar binary

  a4n-llm                   # LLM 请求/响应/provider trait
  a4n-llm-http              # OpenAI-compatible HTTP provider
  a4n-llm-gateway           # LLM sidecar binary，负责 stream/usage/trace metadata，不默认写主库

  a4n-embedding             # embedding trait、模型身份、轻量公共类型
  a4n-embedding-fastembed   # fastembed/hf-hub 本地模型
  a4n-embedding-service     # embedding sidecar binary

  a4n-rag-core              # chunk/filter/search result 等轻量合同
  a4n-rag-index             # Tantivy / LanceDB / vector store 实现
  a4n-rag-service           # RAG sidecar binary

  a4n-image                 # image 请求/响应/provider trait
  a4n-image-providers       # OpenAI/SD WebUI/ComfyUI/CliProxy 实现
  a4n-image-service         # image sidecar binary

  a4n-video                 # video 请求/响应/provider trait
  a4n-video-providers       # NewAPI/Gemini/Seedance/CliProxy 实现
  a4n-video-service         # video sidecar binary

  a4n-import-core           # import 数据结构、解析合同
  a4n-import-service        # import pure transform sidecar，不做业务写库
  a4n-export-core           # export 数据结构、渲染合同
  a4n-export-service        # export pure transform sidecar，不做权限和业务写库

  a4n-services              # 业务服务层；逐步改为调用本地库或 sidecar client
  a4n-orchestrator          # 业务编排 host，管理服务发现、任务、权限、事务边界
  a4n-server                # Axum HTTP 入口
  a4n-tauri                 # Tauri 桌面入口
  a4n-migration             # 迁移工具
  a4n-test-support          # dev-only mock provider/test builder，不进入 release host
```

### 模块拆分归属

当前代码到目标模块的大致映射：

| 当前位置 | 目标位置 | 形态 |
| --- | --- | --- |
| `a4n-engine/src/queue` | `a4n-queue` | crate，可被 host/services 引用 |
| `a4n-engine/src/prompt` | `a4n-prompt` + `a4n-prompt-git` + `a4n-prompt-service` | crate + sidecar |
| `a4n-engine/src/llm` | `a4n-llm` + `a4n-llm-http` + `a4n-llm-gateway` | crate + sidecar |
| `a4n-engine/src/embedding` | `a4n-embedding` + `a4n-embedding-fastembed` + `a4n-embedding-service` | crate + sidecar |
| `a4n-engine/src/rag` | `a4n-rag-core` + `a4n-rag-index` + `a4n-rag-service` | crate + sidecar |
| `a4n-engine/src/image` | `a4n-image` + `a4n-image-providers` + `a4n-image-service` | crate + sidecar |
| `a4n-engine/src/video` | `a4n-video` + `a4n-video-providers` + `a4n-video-service` | crate + sidecar |
| `a4n-services/src/llm_context` | orchestrator llm-context 模块 + `a4n-db` 持久化入口 | LLM gateway 只接收最终请求和返回 usage/trace metadata |
| `a4n-services/src/rag` | `a4n-rag-service` client facade 或 `a4n-orchestrator` adapter | sidecar client |
| `a4n-runtime/src/dynamic_llm.rs` | `a4n-llm-gateway` 或 orchestrator LLM client | sidecar client |
| `a4n-runtime/src/dynamic_embedding.rs` | `a4n-embedding-service` client | sidecar client |
| `a4n-runtime/src/rag_ingest` | `a4n-rag-service` | sidecar |
| `a4n-runtime/src/actions/image_generation` | orchestrator action + `a4n-image-service` client | host + sidecar |
| `a4n-runtime/src/actions/video_generation` | orchestrator action + `a4n-video-service` client | host + sidecar |
| `a4n-migration` | 保持独立 binary | 独立工具 |

### Sidecar 通信协议

所有 sidecar 统一提供：

```text
GET  /healthz
GET  /version
POST /shutdown                 # 仅本地开发/桌面托管可用
POST /rpc                      # JSON-RPC 或 typed HTTP endpoints
```

每个请求必须携带：

```text
request_id
user_id / project_id / root_project_id
runtime_config_snapshot
capability_version
trace_context
```

每个响应必须包含：

```text
status: ok | error
error_code
error_message
trace_id
usage / metrics
artifact_refs
```

错误必须显式失败，不允许 sidecar 不可用时静默返回空结果。host 可以展示“能力未启动/不可用”，但不能伪造成功。

### 服务发现与进程管理

桌面端和开发模式推荐由 host 管理本地 sidecar：

```text
A4N_PORTABLE_HOME/data/services/
  prompt-service.pid
  llm-gateway.pid
  rag-service.pid
  image-service.pid
  video-service.pid
```

配置来源：

```toml
[services.prompt]
mode = "embedded" | "sidecar" | "remote"
url = "http://127.0.0.1:..."
binary = "target/debug/a4n-prompt-service"

[services.rag]
mode = "disabled" | "sidecar" | "remote"
```

服务模式含义：

| mode | 含义 |
| --- | --- |
| `embedded` | 当前进程内库调用，兼容旧路径 |
| `sidecar` | 本地独立进程，支持单独编译/重启 |
| `remote` | 远程服务，适合 Web 部署 |
| `disabled` | 能力不可用，调用时明确失败 |

过渡期允许同一能力同时存在 embedded 和 sidecar 实现，但必须通过同一个 trait/client facade 调用，不能让业务层直接分叉两套逻辑。

### 数据库与事务边界

微服务化后最大的风险是数据库边界。推荐采用分阶段策略：

1. 早期 sidecar 不直接写主业务数据库，只返回结果，由 orchestrator 统一落库。
2. RAG sidecar 可以拥有自己的索引目录和向量库，但主业务记录仍由 host 管理。
3. LLM gateway 默认不写 `llm_call_trace`，只返回 usage、finish_reason、trace metadata；trace 落库由 orchestrator 完成。
4. image/video sidecar 只生成 bytes/metadata/artifact，媒体资产落库由 orchestrator 完成。
5. import/export sidecar 可以读取上传文件/临时目录，但最终项目写入由 orchestrator 事务控制。

推荐边界：

```text
orchestrator owns:
  user/project/auth/permission
  generation_run/workflow_run/checkpoint
  media_asset/media_binding
  task_queue
  business database transaction

sidecar owns:
  provider request
  model loading
  vector/text index
  temporary artifacts
  capability-specific logs/metrics
```

### 开发期构建与重启方式

最终开发命令建议：

```bash
./scripts/dev/check-module.sh prompt
./scripts/dev/check-module.sh llm
./scripts/dev/check-module.sh rag
./scripts/dev/check-module.sh image
./scripts/dev/check-module.sh video

./scripts/dev/run-services.sh prompt,llm,rag
./scripts/dev/restart-service.sh rag
./scripts/dev/run-host.sh tauri
```

内部映射：

```text
prompt -> cargo build -p a4n-prompt-service
llm    -> cargo build -p a4n-llm-gateway
rag    -> cargo build -p a4n-rag-service
image  -> cargo build -p a4n-image-service
video  -> cargo build -p a4n-video-service
host   -> cargo build -p a4n-tauri 或 a4n-server
```

这样才能达成“改 RAG 不重编桌面壳、改图片 provider 不重编 server、改 prompt service 只重启 prompt 服务”的目标。

### 不能避免的重新编译

以下改动仍会触发多模块编译，这是正常且应该保留的约束：

1. `a4n-contracts` 的 RPC/API 合同变化。
2. `a4n-core` 的 SeaORM entity/schema 变化。
3. `a4n-service-protocol` 的错误码、事件、trace contract 变化。
4. 跨 sidecar 的 workflow contract 变化。
5. 安全、权限、数据完整性边界变化。

这些属于系统合同变更，必须全链路检查，不能通过隐藏 fallback 或旧合同兼容绕过去。

## 当前依赖形态

当前主要依赖关系可以概括为：

```text
a4n-tauri
  -> a4n-runtime
  -> a4n-services
  -> a4n-engine
  -> a4n-migration
  -> a4n-core

a4n-server
  -> a4n-runtime
  -> a4n-services
  -> a4n-engine
  -> a4n-migration
  -> a4n-core

a4n-runtime
  -> a4n-services
  -> a4n-engine
  -> a4n-migration
  -> a4n-core

a4n-services
  -> a4n-engine
  -> a4n-core

a4n-engine
  -> a4n-core
  -> prompt / LLM / RAG / embedding / vector DB / image 等实现依赖
```

这意味着：

1. 改 `a4n-core` 会影响几乎所有 crate。
2. 改 `a4n-engine` 会影响 `a4n-services`、`a4n-runtime`、`a4n-tauri`、`a4n-server`。
3. 即使只改 prompt 或 LLM 轻量逻辑，仍可能处在包含 RAG/embedding/vector DB 重依赖的 crate 中。
4. 库 crate 可以单独 `cargo check`，但最终运行仍要重新链接 `a4n-tauri` 或 `a4n-server` 这类 binary crate。

## 设计原则

### 1. 先收窄，后拆分

优先减少不必要直接依赖和默认依赖，再做文件迁移。能通过入口依赖收敛、optional feature 解决的问题，不先做大规模模块搬迁。

### 2. 稳定合同下沉，频繁实现上浮

底层 crate 只放稳定、轻量、跨端共享的合同和基础类型。频繁变化的业务流程、实验性模型、provider 实现、生成流程不应放入最底层。

### 3. 重依赖隔离

`tantivy`、`lancedb`、`arrow-*`、`fastembed`、`hf-hub`、`image`、`git2` 等依赖应尽量被隔离到专门 crate 或 feature 后面。轻量路径不应默认编译这些能力。

### 4. 单一合同来源

重构不能复制业务规则、权限规则、校验规则、缓存规则或 API 合同。移动代码时必须保持原有唯一来源，不能临时保留两套实现。

### 5. 每一步都可验证

每个阶段必须有明确验收命令。验证失败时停在当前阶段，不继续叠加后续结构调整。

## 过渡目标分层

阶段 4-8 的过渡目标结构如下。这里是为降低第一次迁移成本而设计的粗粒度拆分，不是最终 workspace 命名；完整目标以后文 `最终 workspace 划分` 为准。

```text
crates/
  a4n-core           # 既有稳定共享类型、ID、基础错误、轻量合同；不新增高频变化合同
  a4n-prompt         # prompt registry、template loading、Tera 渲染
  a4n-prompt-git     # 可选：prompt Git 版本存储；也可先作为 a4n-prompt feature
  a4n-llm            # LLM provider、HTTP、streaming、usage 解析
  a4n-queue          # 可选：任务队列合同和内存队列实现
  a4n-rag            # 过渡粗粒度：RAG、embedding、全文检索、向量库；长期再拆 core/index/service
  a4n-image          # 图片生成 provider、图片请求/响应模型
  a4n-video-provider # 过渡名称；长期改为 a4n-video + a4n-video-providers + a4n-video-service
  a4n-media          # 只放通用媒体合同/工具，不能重新吸收 image/video provider
  a4n-services       # 业务用例编排
  a4n-runtime        # 状态、配置、任务队列、服务组装
  a4n-migration      # 数据库迁移工具
  a4n-server         # Web server 入口
  a4n-tauri          # 桌面端入口
```

这不是一次性落地目标。实际推进时按阶段逐步靠近。

## 分阶段计划

### 阶段 0：依赖图审计

目标：只分析，不移动代码。

执行内容：

1. 记录当前入口依赖图：

```bash
cargo tree -p a4n-tauri
cargo tree -p a4n-server
cargo tree -p a4n-engine
cargo tree -p a4n-runtime
```

2. 标记重依赖来源：

```text
tantivy
lancedb
arrow-array
arrow-schema
fastembed
hf-hub
image
git2
tauri
sea-orm-migration
```

3. 标记高扇出 crate：

```text
a4n-core
a4n-engine
a4n-services
a4n-runtime
```

4. 输出一份审计结果，说明哪些依赖是必要的，哪些可以通过拆 crate 或 feature 延后编译。

验收标准：

1. 有当前依赖图记录。
2. 有重依赖来源表。
3. 有入口 crate 的直接依赖清单。
4. 未改业务代码。

建议验证：

```bash
./scripts/build.sh --check -p a4n-tauri
./scripts/build.sh --check -p a4n-server
```

### 阶段 1：入口依赖收敛

目标：减少 `a4n-tauri` 和 `a4n-server` 对内部实现 crate 的直接依赖。

当前疑点：

```text
a4n-tauri 直接依赖 a4n-core / a4n-engine / a4n-services / a4n-migration / a4n-runtime
a4n-server 直接依赖 a4n-core / a4n-engine / a4n-services / a4n-migration / a4n-runtime
```

如果某些依赖已经通过 `a4n-runtime` 或 `a4n-services` 间接提供，入口 crate 应优先通过 runtime/service facade 使用能力，而不是越层依赖 engine。

执行内容：

1. 用 `rg` 检查入口 crate 中的跨层 import：

```bash
rg "a4n_(core|engine|services|migration)::" crates/a4n-tauri crates/a4n-server
```

2. 对只用于类型引用的依赖，判断是否属于既有稳定共享类型；既有类型可暂留 `a4n-core`，新增 API/RPC 合同优先进入 `a4n-api-contract`、`a4n-service-protocol` 或 `a4n-contracts`。
3. 对只用于业务调用的依赖，优先改为通过 `a4n-runtime` 或 `a4n-services` 暴露的 facade 调用。
4. 删除入口 `Cargo.toml` 中未实际使用的依赖。
5. 对入口层直接引用 `a4n_engine::embedding::provider::DownloadProgress` 这类类型的情况，优先把类型移动到更稳定的公共接口层，或由 runtime action 暴露包装类型。

不做的事：

1. 不改业务行为。
2. 不新增第二套路由、命令或权限逻辑。
3. 不为了移除依赖而把内部实现暴露成不稳定 public API。

验收标准：

1. 入口 crate 的直接依赖减少，或有明确理由保留。
2. `cargo check -p a4n-tauri` 通过。
3. `cargo check -p a4n-server` 通过。
4. API parity 检查不受影响。

建议验证：

```bash
timeout 60 cargo check -p a4n-tauri
timeout 60 cargo check -p a4n-server
cd frontend && npm run check:web-api-parity
```

### 阶段 2：迁移能力从运行时默认路径剥离

目标：避免普通运行时检查默认编译迁移工具链。

当前疑点：

```text
a4n-runtime -> a4n-migration
a4n-tauri   -> a4n-migration
a4n-server  -> a4n-migration
```

迁移能力通常有两种合理形态：

1. 启动前独立执行 `a4n-migration`。
2. 入口启用 `migration` feature，由 runtime 在启动阶段执行。

推荐优先方案：把 `a4n-runtime` 对 `a4n-migration` 的依赖改成 optional feature。

候选配置：

```toml
# crates/a4n-runtime/Cargo.toml
[features]
default = []
migration = ["dep:a4n-migration"]

[dependencies]
a4n-migration = { path = "../a4n-migration", optional = true }
```

执行内容：

1. 找出 runtime 中使用 migration 的位置。
2. 把 `AppState::ensure_compatible_schema` 中的 `ALTER TABLE` / `CREATE INDEX` 收敛回 `a4n-migration`。
3. 将 `ensure_compatible_schema` 改成只读兼容性检查，或在迁移完成后删除。
4. 用 `#[cfg(feature = "migration")]` 包住迁移入口。
5. 对未启用 feature 的路径给出明确错误或要求调用独立迁移命令，不做静默跳过。
6. 在 `a4n-tauri` / `a4n-server` 中按实际启动策略决定是否启用 `a4n-runtime/migration`。

不做的事：

1. 不改变迁移内容。
2. 不改变 migration crate 的数据库 schema 行为。
3. 不把迁移失败改成 warning 或静默成功。
4. 不继续在 runtime 中新增 schema 修改 SQL。

验收标准：

1. `cargo check -p a4n-runtime --no-default-features` 可以通过，或明确说明 runtime 当前必须带 migration。
2. 启用 migration feature 后原启动/迁移行为保持。
3. `a4n-migration` 仍可作为独立工具运行。
4. `rg "ALTER TABLE|CREATE INDEX|CREATE TABLE" crates/a4n-runtime/src/state.rs` 不再出现新增 schema 修改，或只剩兼容性检查说明。

建议验证：

```bash
timeout 60 cargo check -p a4n-runtime --no-default-features
timeout 60 cargo check -p a4n-runtime --features migration
timeout 60 cargo check -p a4n-migration
```

### 阶段 3：任务队列边界收敛

目标：先处理 `a4n-engine::queue` 的公共合同角色，避免后续拆 engine 时被队列类型卡住。

当前使用点：

```text
a4n_services::queue::PersistentTaskQueue
  -> a4n_engine::queue::memory_queue::MemoryTaskQueue
  -> a4n_engine::queue::priority::{TaskQueue, TaskType, TaskPriority, TaskStatus, TaskInfo}

a4n_runtime::dynamic_llm / dynamic_embedding / actions
  -> a4n_engine::queue::priority::{TaskPermit, TaskQueue, TaskType, TaskPriority}
```

候选方案：

1. 新建 `a4n-queue`，放 `priority` 合同和 `memory_queue` 实现，`a4n-services` 和 `a4n-runtime` 都依赖它。
2. 仅把已经稳定且必须跨端一致的队列事件映射暂留 `a4n-core::event`；队列合同不再新下沉到 `a4n-core`，`MemoryTaskQueue` 实现迁到 `a4n-queue` 或 `a4n-services::queue`。
3. 短期保持队列在 `a4n-engine`，但文档明确 `a4n-engine` 不能拆空；后续拆 prompt/LLM/RAG 时接受 engine 仍作为 queue 容器存在。

推荐方案：新建 `a4n-queue`。原因是队列合同既不是数据库 entity，也不是 AI provider，实现上又被 services/runtime 同时使用，独立 crate 边界更清晰。

执行内容：

1. 新建 `crates/a4n-queue`。
2. 迁移 `crates/a4n-engine/src/queue` 到 `a4n-queue`。
3. 更新 `a4n-services`、`a4n-runtime`、`a4n-tauri`、`a4n-server` 中的队列 import。
4. `a4n-engine` 如果仍有旧路径需求，可短期 re-export `a4n_queue`，但新代码直接使用 `a4n_queue`。
5. 删除 `a4n-engine` 对 queue 模块的 ownership，或明确留下短期兼容层。

不做的事：

1. 不改变任务优先级、状态枚举和并发限制语义。
2. 不改变 `PersistentTaskQueue` 的数据库持久化行为。
3. 不改变任务恢复、中断、取消语义。

验收标准：

1. `cargo check -p a4n-queue` 通过。
2. `cargo check -p a4n-services` 通过。
3. `cargo check -p a4n-runtime` 通过。
4. `rg "a4n_engine::queue" crates` 不再命中新代码路径，或仅命中过渡 re-export/兼容说明。

建议验证：

```bash
timeout 60 cargo check -p a4n-queue
timeout 60 cargo check -p a4n-services
timeout 60 cargo check -p a4n-runtime
timeout 60 cargo check -p a4n-tauri
```

### 阶段 4：拆出 `a4n-prompt`

目标：先从 `a4n-engine` 中拆出边界较清楚、依赖较轻的 prompt 能力，作为后续拆分模板。

候选范围：

```text
crates/a4n-engine/src/prompt/
prompt registry
Tera engine
template loader
prompt mutation / validation
```

目标依赖：

```text
a4n-prompt
  -> a4n-core
  -> tera
  -> git2        # 第一阶段允许保留；后续可迁入 a4n-prompt-git 或 optional feature
  -> serde
  -> serde_json
  -> serde_yaml
  -> anyhow / thiserror
  -> tracing
  -> chrono
  -> regex
  -> similar
  -> parking_lot
  -> tokio
```

`a4n-prompt` 不应依赖：

```text
lancedb
fastembed
tantivy
arrow-array
arrow-schema
image
hf-hub
tauri
axum
```

注意：`git2` 当前来自 prompt 版本存储，不应和 RAG/embedding 这类重依赖混为一类。第一阶段可以接受 `a4n-prompt` 带 `git2`，但必须在文档中标注这是 prompt 持久化带来的成本。

执行内容：

1. 新建 `crates/a4n-prompt`。
2. 将 prompt 相关模块从 `a4n-engine` 迁移到 `a4n-prompt`。
3. 在 `a4n-engine` 中保留兼容 re-export，或直接修改调用方 import，二者选其一。优先避免长期 re-export。
4. 更新 workspace `Cargo.toml`。
5. 更新 `a4n-engine` / `a4n-services` / `a4n-runtime` 中的 prompt import。
6. 保持 prompt 文件路径、registry 语义、模板渲染错误行为不变。
7. 保持 `create_prompt_engine` 中 Git store 初始化行为不变，除非本阶段明确选择 optional feature 方案。

不做的事：

1. 不重写 prompt DSL。
2. 不改变 prompt registry 格式。
3. 不改变 LLM 请求构造行为。
4. 不同时拆 LLM、RAG、embedding。

验收标准：

1. `cargo check -p a4n-prompt` 通过。
2. `cargo tree -p a4n-prompt` 不包含 RAG/embedding/vector DB/image 相关重依赖。
3. 如果 `a4n-prompt` 仍包含 `git2`，必须在验收记录中说明原因。
4. `cargo check -p a4n-engine` 通过。
5. 依赖 prompt 的业务测试或最小 check 通过。

建议验证：

```bash
timeout 60 cargo check -p a4n-prompt
cargo tree -p a4n-prompt
timeout 60 cargo check -p a4n-engine
timeout 60 cargo check -p a4n-services
timeout 60 cargo check -p a4n-tauri
```

### 阶段 5：拆出 `a4n-llm`

目标：把 LLM provider、HTTP、streaming、usage 解析从 `a4n-engine` 中拆出，让 LLM 变更不牵动 RAG/embedding/vector DB。

候选范围：

```text
crates/a4n-engine/src/llm/
ChatRequest / ChatResponse 相关类型
Provider trait
HTTP provider
streaming 解析
usage / cache token 解析
```

需要先确认的边界：

1. `ChatRequest` / `ChatResponse` 是否属于稳定合同。如果多个层都需要直接构造请求，优先放在 `a4n-llm`；跨 sidecar RPC 需要的稳定部分再映射到 `a4n-service-protocol` / `a4n-contracts`，不把新增 LLM 合同继续下沉到 `a4n-core`。
2. `DynamicLlmProvider` 当前在 runtime 中的位置是否依赖 runtime state，不应被强行下沉。
3. LLM trace、context manifest、checkpoint helper 和 provider 调用之间的边界是否清晰。

执行内容：

1. 新建 `crates/a4n-llm`。
2. 迁移 provider trait、HTTP provider、请求/响应模型。
3. `a4n-engine` 如果仍负责更高层 agent/pipeline，可以依赖 `a4n-llm`。
4. `a4n-runtime` 的 dynamic provider 根据实际边界依赖 `a4n-llm`。

不做的事：

1. 不改 provider API 合同。
2. 不改模型配置读取规则。
3. 不改 trace 落库合同。

验收标准：

1. `cargo check -p a4n-llm` 通过。
2. `a4n-llm` 不依赖 RAG/embedding/vector DB。
3. LLM provider 相关测试通过。
4. 业务链路仍通过 `a4n-services` 或 `a4n-runtime` 调用 LLM，不新增绕过 trace 的调用入口。

建议验证：

```bash
timeout 60 cargo check -p a4n-llm
cargo tree -p a4n-llm
timeout 60 cargo test -p a4n-engine llm -- --nocapture
timeout 60 cargo check -p a4n-runtime
```

### 阶段 6：拆出 `a4n-rag`

目标：隔离最重的 RAG、embedding、全文检索、向量库依赖。

候选范围：

```text
RAG ingest
RAG search
embedding model
local embedding
Tantivy index
LanceDB vector store
fastembed / hf-hub integration
```

目标依赖：

```text
a4n-rag
  -> a4n-core
  -> a4n-llm  # 仅在确实需要 LLM 辅助时
  -> tantivy
  -> lancedb
  -> arrow-array
  -> arrow-schema
  -> fastembed
  -> hf-hub
```

执行前置：

1. 先调整 `AppState` 初始化策略，避免 minimal profile 仍无条件初始化 LanceDB/Tantivy。
2. 再定义 trait 边界，避免 services 直接依赖具体 vector DB：

```rust
trait EmbeddingService
trait RagSearchService
trait RagIngestService
```

3. trait 放置位置要谨慎。如果 trait 只被业务层和实现层共享，可放 `a4n-services` 的接口模块或独立轻量 crate；如果需要跨 runtime/server/tauri/sidecar 共享，优先放 `a4n-rag-core`、`a4n-capabilities` 或 `a4n-service-protocol`，只有低频稳定基础枚举才考虑暂留 `a4n-core`。
4. 配置、缓存目录、模型加载错误必须有明确边界。
5. `RagService` 当前直接持有 `Arc<dyn VectorStore>`，拆分时要决定它继续作为服务层 facade，还是迁入 `a4n-rag`。

不做的事：

1. 不改检索结果排序语义。
2. 不改 embedding 模型默认选择。
3. 不把 RAG 失败改成空结果静默返回。
4. 不在拆分同时重写索引格式。

验收标准：

1. 不使用 RAG 的入口可以不编译 `a4n-rag`。
2. 使用 RAG 的入口启用对应 feature 后行为不变。
3. RAG 相关测试通过。
4. `cargo tree -p a4n-tauri --no-default-features` 能体现重依赖是否被隔离。

建议验证：

```bash
timeout 60 cargo check -p a4n-rag
cargo tree -p a4n-rag
timeout 60 cargo test -p a4n-runtime rag -- --nocapture
timeout 60 cargo check -p a4n-tauri
timeout 60 cargo check -p a4n-server
```

### 阶段 7：拆出 image/video provider

目标：在 prompt、LLM、RAG 边界稳定后，把图片和视频 provider 从 `a4n-engine` 中拆出。

候选拆分：

```text
a4n-image
  -> ImageGenerationRequest
  -> ImageGenerationResult
  -> ImageReference
  -> ImageProvider
  -> pick_image_provider
  -> OpenAI / SD WebUI / ComfyUI / CliProxy 等 provider

a4n-video-provider
  -> VideoGenerationRequest
  -> VideoGenerationResult
  -> VideoJobState
  -> VideoProvider
  -> pick_video_provider
  -> provider request preview helpers
  -> NewAPI / Gemini Veo / Seedance / CliProxy 等 provider
```

执行前置：

1. 先把 `VideoGenerationRequest` / `ImageGenerationRequest` 是否属于稳定合同判断清楚。
2. 对 request preview 中直接调用 provider static helper 的代码建立清晰边界。
3. 避免在拆 provider 时顺手重写视频生成流程。

不做的事：

1. 不改变 provider 请求体。
2. 不改变轮询状态语义。
3. 不改变图片/视频配置读取规则。
4. 不同时重构大型 `video_generation` action。

验收标准：

1. `cargo check -p a4n-image` 通过。
2. `cargo check -p a4n-video-provider` 通过。
3. 图片生成、视频生成相关 runtime check 通过。
4. request preview 输出保持一致。

建议验证：

```bash
timeout 60 cargo check -p a4n-image
timeout 60 cargo check -p a4n-video-provider
timeout 60 cargo check -p a4n-runtime
```

### 阶段 8：feature profiles 与开发命令固化

目标：把不同开发场景固化成明确命令，减少手工记忆。

候选 feature profile：

```text
desktop-minimal
desktop-full
server-minimal
server-full
rag
media
migration
```

候选脚本入口：

```bash
./scripts/build.sh --check -p a4n-tauri
./scripts/build.sh --check -p a4n-server
./scripts/build.sh --check -p a4n-prompt
./scripts/build.sh --check -p a4n-llm
./scripts/build.sh --backend --all
```

如果需要进一步封装，可以新增：

```bash
./scripts/dev/check-rust.sh prompt
./scripts/dev/check-rust.sh desktop
./scripts/dev/check-rust.sh server
./scripts/dev/check-rust.sh full
```

不做的事：

1. 不让脚本隐藏真实失败。
2. 不让 minimal profile 成为无法运行的假成功路径。
3. 不把 release 构建默认改成裁剪能力的构建。
4. 不指望 feature 单独解决编译牵连；如果入口 full profile 默认启用所有能力，仍需要 crate 拆分。

验收标准：

1. 日常 check 命令清晰。
2. CI 或交付前仍有 full check/build。
3. 文档和脚本帮助一致。

### 阶段 9：能力 facade 与 sidecar 协议层

目标：在拆独立进程前，先把 host 内部调用收敛到能力 facade，并建立稳定协议和 client/server 基础设施，避免每个服务各自定义通信方式。

新增 crate：

```text
a4n-capabilities
a4n-service-protocol
```

`a4n-capabilities` 职责：

```text
PromptClient trait
LlmClient trait
EmbeddingClient trait
RagClient trait
ImageClient trait
VideoClient trait
ServiceClients / A4nCapabilities
embedded client implementation
sidecar client implementation placeholder
```

`a4n-service-protocol` 职责：

```text
ServiceRequest<T>
ServiceResponse<T>
ServiceError
ServiceHealth
ServiceVersion
TraceContext
RuntimeConfigSnapshotRef
CapabilityVersion
ServiceTaskEvent
ArtifactRef
```

执行内容：

1. `AppState` 新增 `capabilities` 字段。
2. `state.llm`、`state.engine`、`state.embedding`、`state.rag` 等直接访问逐步改为 capability client。
3. `ServerState` 逐步减少 `Deref<Target = AppState>` 带来的隐式访问，route 调用 runtime/orchestrator facade。
4. `runtime_config_snapshot`、`runtime_user_id` 从 task-local 读取改为显式注入到 sidecar request context。
5. 建立 canonical `TaskKind`，收敛 `a4n_core::event::channel::TaskType` 和 `a4n_engine/a4n_queue::queue::TaskType` 的映射。
6. 定义 `ServiceTaskEvent`，把 sidecar 进度、chunk、完成、失败、取消确认映射回现有 SSE/Tauri 事件。
7. 定义 `ArtifactRef`，不让 sidecar 随意返回本地绝对路径。

协议要求：

1. 所有 sidecar 请求必须有 `request_id` 和 `trace_context`。
2. 错误使用稳定 `error_code`，不能只返回字符串。
3. 响应区分业务失败、配置失败、能力未启用、服务不可用、用户取消。
4. 大文件和二进制产物优先传 artifact ref/path，不在 JSON 里传大字节。
5. 协议版本必须可检查，host 发现版本不兼容时显式失败。
6. runtime config、用户上下文、能力版本必须显式传递，不能依赖 task-local。

不做的事：

1. 不直接把所有业务 DTO 搬进协议层。
2. 不让 sidecar 绕过 host 权限校验。
3. 不在协议层引入数据库访问。
4. 不一次性把所有 action 从 `&AppState` 改掉；先改 provider 能力访问路径。

验收标准：

1. `cargo check -p a4n-capabilities` 通过。
1. `cargo check -p a4n-service-protocol` 通过。
2. 至少 LLM 或 prompt 调用已通过 capability facade 访问 embedded 实现。
3. 有一个最小 health/version server 和 client smoke test。
4. 协议错误码有文档说明。
5. `rg "state\\.(llm|engine|embedding|rag)" crates/a4n-runtime/src` 的命中数开始下降，并有迁移清单。

建议验证：

```bash
timeout 60 cargo check -p a4n-capabilities
timeout 60 cargo check -p a4n-service-protocol
timeout 60 cargo test -p a4n-service-protocol
timeout 60 cargo check -p a4n-runtime
```

### 阶段 10：sidecar 进程管理与服务发现

目标：让 host 可以以统一方式启动、停止、发现本地服务，并支持连接远程服务。

新增 crate：

```text
a4n-service-manager
```

职责：

```text
读取 services 配置
分配本地端口
启动 sidecar binary
记录 pid/port
健康检查
优雅停止
失败重启策略
remote URL 连接
```

配置示例：

```toml
[services.prompt]
mode = "embedded" # embedded | sidecar | remote | disabled
binary = "target/debug/a4n-prompt-service"
url = "http://127.0.0.1:0"

[services.rag]
mode = "sidecar"
binary = "target/debug/a4n-rag-service"
```

执行内容：

1. `a4n-runtime` 增加 `ServiceRegistry`，统一保存 embedded/sidecar/remote client。
2. Tauri dev 模式可以自动拉起本地 sidecar。
3. server 模式默认不自动启动本地 sidecar，优先读取环境或配置。
4. 服务不可用时返回明确错误，不静默回退到 embedded，除非配置显式允许 fallback。

不做的事：

1. 不默认杀掉非本次启动的服务进程。
2. 不把服务重启失败吞掉。
3. 不让远程服务共享本地文件路径，远程模式必须使用上传/artifact 协议。

验收标准：

1. 能启动一个 demo sidecar 并通过 health check。
2. sidecar 退出后 host 能给出明确错误。
3. 配置为 `disabled` 时调用该能力明确失败。

建议验证：

```bash
timeout 60 cargo check -p a4n-service-manager
timeout 60 cargo check -p a4n-runtime
```

### 阶段 11：prompt 和 LLM 先 sidecar 化

目标：先把边界较清晰且使用频率高的 prompt/LLM 做成可独立编译、独立重启的服务。

新增 binary：

```text
a4n-prompt-service
a4n-llm-gateway
```

prompt service API：

```text
render_prompt
get_prompt
list_prompts
update_prompt
update_many_prompts
reset_prompt
version_history
diff_prompt
validate_variables
hydrate_project_overrides
```

LLM gateway API：

```text
chat
chat_stream
test_connection
preview_effective_budget
normalize_request_options
```

执行内容：

1. 先保留 embedded 实现，新增 sidecar client，实现同一 trait。
2. `a4n-services` 和 `a4n-runtime` 只依赖 trait/client facade，不直接依赖具体 provider。
3. `DynamicLlmProvider` 中和 provider 调用、usage 解析、finish_reason 规范化相关逻辑迁入 LLM gateway；trace/context manifest 落库逻辑留在 orchestrator，或拆成共享库后仍由 orchestrator 调用。
4. prompt Git store 随 prompt service 运行，host 不直接操作 prompt git 仓库。
5. 流式 LLM 使用 SSE 或 chunked JSON lines，必须保留 usage 和取消语义。

不做的事：

1. 不改变 prompt registry 文件格式。
2. 不改变 LLM 请求/响应业务合同。
3. 不绕过 `llm_call_trace`。
4. 不把 prompt/LLM 错误变成 fallback 文本。

验收标准：

1. `cargo build -p a4n-prompt-service` 后可单独启动并响应 render 请求。
2. `cargo build -p a4n-llm-gateway` 后可单独启动并完成 mock/test connection。
3. host 使用 embedded 与 sidecar 两种模式时，核心请求摘要一致。
4. 改 prompt service 不需要重新编译 `a4n-tauri`。
5. 改 LLM gateway 不需要重新编译 `a4n-server`，只需重启 gateway。

建议验证：

```bash
timeout 60 cargo check -p a4n-prompt-service
timeout 60 cargo check -p a4n-llm-gateway
timeout 60 cargo check -p a4n-runtime
timeout 60 cargo check -p a4n-tauri
```

### 阶段 12：embedding 与 RAG sidecar 化

目标：把最重的本地模型、向量库和全文检索依赖移出 host。

新增 binary：

```text
a4n-embedding-service
a4n-rag-service
```

embedding service API：

```text
embed
embed_with_identity
model_identity
download_model
list_local_models
delete_local_model
```

RAG service API：

```text
prepare_ingest
ingest_chunks
delete_project_index
hybrid_search
hybrid_search_with_embedding
rebuild_index
index_status
```

执行内容：

1. `a4n-runtime/src/dynamic_embedding.rs` 改成 embedded/sidecar client facade。
2. `a4n-runtime/src/rag_ingest` 迁到 RAG service 或改为调用 RAG client。
3. `RagService` 不再持有具体 `VectorStore`，而是持有 `RagClient` trait。
4. LanceDB/Tantivy/index path 由 RAG service 管理。
5. embedding 模型下载进度通过统一 service progress event 回传。

不做的事：

1. 不让 RAG service 直接修改项目主表。
2. 不改变检索排序语义。
3. 不在 RAG 不可用时返回空上下文假成功。
4. 不把本地文件路径直接传给远程 RAG 服务。

验收标准：

1. host minimal profile 不编译 `lancedb`、`tantivy`、`fastembed`、`hf-hub`。
2. `cargo build -p a4n-rag-service` 可单独编译重依赖。
3. 重启 RAG service 后 host 不需要重新编译。
4. RAG 搜索和入库行为与 embedded 模式一致。

建议验证：

```bash
timeout 60 cargo check -p a4n-embedding-service
timeout 60 cargo check -p a4n-rag-service
cargo tree -p a4n-tauri --no-default-features
timeout 60 cargo check -p a4n-runtime
```

### 阶段 13：image/video sidecar 化

目标：把 provider 请求体、轮询、预览、下载等图像/视频生成能力移出 host。

新增 binary：

```text
a4n-image-service
a4n-video-service
```

image service API：

```text
generate_image
preview_image_request
test_image_config
```

video service API：

```text
generate_video
poll_video_job
preview_video_request
test_video_config
download_video_artifact
```

执行内容：

1. `a4n-services/src/image/provider_config.rs` 改为 provider client factory。
2. `a4n-services/src/video/config.rs` 改为 provider client factory。
3. `a4n-runtime/src/actions/image_generation` 只负责业务校验、任务、落库和媒体绑定。
4. `a4n-runtime/src/actions/video_generation` 只负责业务校验、任务、落库和媒体绑定。
5. provider-specific request preview helper 移入 image/video service。
6. 大文件产物通过 artifact path 或临时文件引用返回，host 再统一入库。

不做的事：

1. 不改变已有 provider 请求体。
2. 不改变视频轮询状态映射。
3. 不改变媒体资产归属和权限校验。
4. 不在 sidecar 中直接写主业务媒体表。

验收标准：

1. 改 image provider 只编译 `a4n-image-service`。
2. 改 video provider 只编译 `a4n-video-service`。
3. host minimal profile 不编译 provider-specific 重依赖。
4. request preview 在 embedded/sidecar 模式下输出一致。

建议验证：

```bash
timeout 60 cargo check -p a4n-image-service
timeout 60 cargo check -p a4n-video-service
timeout 60 cargo check -p a4n-runtime
```

### 阶段 14：workflow/orchestrator 收敛

目标：把 host 从“拥有所有实现”收敛成“业务编排 + sidecar client”，让各能力模块真正可单独替换。

新增或强化：

```text
a4n-orchestrator
a4n-db::workflow
```

职责划分：

```text
a4n-orchestrator:
  权限校验
  项目/用户上下文
  workflow_run / generation_run / checkpoint
  task_queue
  sidecar 调用
  事务落库

sidecar:
  provider 调用
  模型加载
  请求预览
  索引/临时 artifact
```

执行内容：

1. 把 `a4n-runtime/src/actions/*` 中 provider 直接调用改成 orchestrator 调用 client。
2. 保留业务事务在 orchestrator，不分散到多个 sidecar。
3. workflow checkpoint 的 `call_key` 和 `input_fingerprint` 作为跨服务幂等键。
4. 统一 progress event，让 sidecar 进度能映射回现有 SSE/Tauri progress。
5. `a4n-tauri` 和 `a4n-server` 只调用 orchestrator facade。

不做的事：

1. 不让 sidecar 自行判断用户是否有项目权限。
2. 不让多个服务同时写同一业务事务。
3. 不把 workflow 状态拆散到多个数据库中。

验收标准：

1. `a4n-tauri` 不再直接依赖 provider crates。
2. `a4n-server` 不再直接依赖 provider crates。
3. 修改 RAG/image/video/LLM provider 不需要重编 host。
4. 修改业务合同或 orchestrator 逻辑才需要重编 host。

建议验证：

```bash
timeout 60 cargo check -p a4n-orchestrator
timeout 60 cargo check -p a4n-tauri
timeout 60 cargo check -p a4n-server
cargo tree -p a4n-tauri
cargo tree -p a4n-server
```

### 阶段 15：开发脚本与交付形态

目标：让“只编译变更服务并重启”成为默认开发路径。

新增脚本：

```bash
scripts/dev/check-module.sh
scripts/dev/build-service.sh
scripts/dev/run-services.sh
scripts/dev/restart-service.sh
scripts/dev/run-host.sh
scripts/dev/service-status.sh
```

命令示例：

```bash
./scripts/dev/build-service.sh rag
./scripts/dev/restart-service.sh rag
./scripts/dev/build-service.sh video
./scripts/dev/restart-service.sh video
./scripts/dev/run-host.sh tauri
```

交付形态：

```text
desktop bundle:
  a4n-tauri
  services/a4n-prompt-service
  services/a4n-llm-gateway
  services/a4n-embedding-service
  services/a4n-rag-service
  services/a4n-image-service
  services/a4n-video-service

server deployment:
  a4n-server
  a4n-orchestrator
  sidecar services 或 remote services
```

验收标准：

1. 开发者改某个服务后，有明确一条命令构建并重启该服务。
2. host 不因服务重启丢失前端 session。
3. service status 能显示 pid、port、version、health。
4. release bundle 能包含并启动所需 sidecar。

### 阶段 16：移除 embedded 默认路径

目标：当 sidecar 模式稳定后，逐步移除重能力的 embedded 默认路径，让 host 真正瘦身。

执行内容：

1. prompt/LLM 可保留显式配置的 embedded 测试/离线模式，但默认走 sidecar，且不做静默 fallback。
2. RAG/embedding/image/video 默认走 sidecar。
3. host minimal profile 不再包含 RAG/embedding/image/video provider 实现。
4. full profile 用于 CI 和回归，不作为日常开发默认路径。
5. 文档和脚本删除旧的隐式 fallback 描述。

验收标准：

1. `cargo tree -p a4n-tauri --no-default-features` 不包含 `lancedb`、`tantivy`、`fastembed`、`hf-hub`、重 provider crates。
2. 修改 sidecar 服务代码不触发 host 编译。
3. host 在 sidecar 不可用时明确失败并提示服务状态。
4. 全量交付构建仍可一次性构建所有服务和 host。

## 代码放置规则

### `a4n-core`

适合放：

```text
稳定 ID 类型
既有稳定 DTO / API contract（暂留，不作为新增合同默认落点）
共享错误类型
事件结构
轻量领域模型
跨端必须一致的枚举
```

新增跨进程 RPC 合同优先放 `a4n-service-protocol` / `a4n-contracts`；新增 Tauri/Web API 合同优先放 `a4n-api-contract`。不要继续把高频变化合同下沉到 `a4n-core`。

不适合放：

```text
具体业务流程
生成流水线
provider 实现
数据库访问实现
RAG / embedding 实现
频繁变化的实验性模型
```

### `a4n-engine`

长期应收敛为更高层引擎编排或逐步拆空。它不应继续无限吸收所有 AI 能力。

短期保留：

```text
尚未拆分的 agent / pipeline / engine 逻辑
暂时需要共同演进的 AI 编排代码
```

优先拆出：

```text
prompt
llm
rag
media
```

### `a4n-services`

适合放：

```text
业务用例
权限和数据完整性边界
服务层校验
跨 repository / provider 的编排
```

不适合放：

```text
Tauri command 适配
HTTP route 适配
具体向量库实现细节
具体桌面窗口逻辑
```

### `a4n-runtime`

适合放：

```text
运行时状态
配置快照
任务队列
服务组装
启动流程
跨服务 action facade
```

不适合放：

```text
数据库迁移实现细节
HTTP route 逻辑
Tauri command 参数适配
具体 prompt 模板解析实现
```

### `a4n-tauri` / `a4n-server`

适合放：

```text
入口启动
协议适配
命令或路由参数解析
认证上下文提取
调用 runtime/service facade
```

不适合放：

```text
业务规则
权限规则的第二份实现
provider 细节
RAG/embedding 细节
数据库 schema 迁移
```

## 验收矩阵

每次重构 PR 或工作批次至少选择对应验证：

| 改动范围 | 必跑验证 |
| --- | --- |
| 只改脚本 | `bash -n scripts/build.sh`，目标脚本 smoke test |
| 只改 `a4n-core` | `timeout 60 cargo check -p a4n-core`，再检查一个上层入口 |
| 改 prompt | `timeout 60 cargo check -p a4n-prompt`，`timeout 60 cargo check -p a4n-engine` |
| 改 LLM provider | `timeout 60 cargo check -p a4n-llm`，相关 provider 测试 |
| 改 RAG/embedding | `timeout 60 cargo check -p a4n-rag`，RAG 相关测试 |
| 改 prompt sidecar | `timeout 60 cargo check -p a4n-prompt-service`，render smoke test |
| 改 LLM gateway | `timeout 60 cargo check -p a4n-llm-gateway`，chat/mock stream smoke test |
| 改 embedding sidecar | `timeout 60 cargo check -p a4n-embedding-service`，embedding mock/local smoke test |
| 改 RAG sidecar | `timeout 60 cargo check -p a4n-rag-service`，search/ingest smoke test |
| 改 image sidecar | `timeout 60 cargo check -p a4n-image-service`，request preview smoke test |
| 改 video sidecar | `timeout 60 cargo check -p a4n-video-service`，request preview/poll smoke test |
| 改 service protocol | `timeout 60 cargo check -p a4n-service-protocol`，所有 sidecar client check |
| 改 services | `timeout 60 cargo check -p a4n-services`，目标服务测试 |
| 改 runtime | `timeout 60 cargo check -p a4n-runtime`，目标 action 测试 |
| 改 Tauri command | `timeout 60 cargo check -p a4n-tauri` |
| 改 server route | `timeout 60 cargo check -p a4n-server`，`cd frontend && npm run check:web-api-parity` |
| 改前端 | `cd frontend && npm run build` |
| 交付前 | full host check、所有 sidecar check、前端 build、service health smoke test |

## 风险与控制

### 风险 1：大范围移动文件导致冲突

控制：

1. 每阶段只移动一个能力域。
2. 不同时做业务逻辑重写。
3. 对已有大量未提交改动的工作区，优先做只读审计和小范围依赖收敛。

### 风险 2：re-export 形成长期兼容层

控制：

1. 如果使用 re-export，只作为迁移过渡。
2. 文档中标注待删除时间点。
3. 新代码必须直接依赖新 crate，不再走旧路径。

### 风险 3：feature gate 产生假成功

控制：

1. 只有真实可选能力才 feature 化。
2. 关闭 feature 时必须明确不可用错误，不能静默返回空结果。
3. full profile 必须长期保留并定期验证。

### 风险 4：接口边界过早抽象

控制：

1. 先从真实调用点提取接口。
2. 不为未来未知 provider 设计大接口。
3. trait 方法数量保持最小，失败路径显式。

### 风险 5：`a4n-core` 继续膨胀

控制：

1. 新增 core 模型前先判断是否稳定跨端共享。
2. 实验性 feature 模型先放在 feature 所属 crate。
3. API/RPC 合同稳定后优先进入 `a4n-api-contract`、`a4n-service-protocol` 或 `a4n-contracts`；只有低频基础共享类型才允许进入 `a4n-core`。

## 执行路线

### 近期路线

近期先打通边界，为完整微服务化做准备：

1. 阶段 0：依赖图审计。
2. 阶段 1：入口依赖收敛。
3. 阶段 2：迁移能力 feature 化或独立化。
4. 阶段 3：任务队列边界收敛。
5. 阶段 4：拆 `a4n-prompt`。
6. 阶段 5：拆 `a4n-llm`。
7. 阶段 8：feature profiles 与开发命令固化。
8. 阶段 9：建立 capability facade 和 sidecar 协议层。
9. 阶段 10：建立 sidecar 进程管理。

这条路线完成后，项目具备服务化基础，但还没有完全达成“改哪个能力只重启哪个能力”的最终目标。

### 完整微服务化路线

完整目标按以下顺序推进：

1. 阶段 0-3：完成依赖审计、入口收敛、migration 剥离、queue 独立。
2. 阶段 4-7：把 prompt、LLM、RAG、embedding、image、video 从 `a4n-engine` 拆成独立能力 crate。
3. 阶段 8-10：建立 feature profiles、capability facade、统一 sidecar 协议、服务发现和进程管理。
4. 阶段 11：prompt/LLM 先 sidecar 化，验证服务化路径。
5. 阶段 12：embedding/RAG sidecar 化，移出最重依赖。
6. 阶段 13：image/video sidecar 化，移出 provider 请求和轮询。
7. 阶段 14：orchestrator 收敛，host 只保留入口和业务编排。
8. 阶段 15：开发脚本与交付包固化。
9. 阶段 16：移除重能力 embedded 默认路径。

如果只做第 0 到第 4 阶段，预期改动面为中等偏小到中等，主要是 `Cargo.toml`、模块路径、少量 import、队列模块迁移和 prompt 相关模块移动。

如果进入 `a4n-rag` 或 image/video provider 拆分，改动面会明显变大，应单独排期，不和 prompt/LLM 拆分混在同一批。

如果推进到阶段 16，才能基本达成目标：修改某个重能力模块后，只编译对应 sidecar 并重启该服务，host 不需要重新编译。

## 完成定义

### 2026-07 收尾审计结论

本轮在阶段性重构完成后又执行了一次以“真实消费者”为准的反向审计，补齐了原计划未覆盖的收尾条件：

1. 删除无生产消费者的 service/orchestrator 兼容壳、重复 CRUD、重复事务 facade 和只供旧测试入口使用的生产 API；测试需要直接验证 DB 合同时改走权威 DB 入口。
2. sidecar profile 缺少 LLM 或 prompt capability 时必须明确报错，不能落回 embedded provider/engine；Full/Minimal 只有在显式 profile 语义下才使用 embedded 路径。
3. 运行模式只在环境变量未设置时使用当前 build 的默认 profile；`A4N_RUNTIME_PROFILE` 已设置为空、非 Unicode 或未知值时必须阻止启动，不能把配置错误解释为 build 默认值。
4. 删除从未接线且没有第二消费者的 `a4n-workflow-core`；workflow/checkpoint 持久化合同由 `a4n-db::workflow` 唯一持有，执行者为 `a4n-orchestrator`。
5. `OrchestratorContext` 公开实例方法必须存在真实接收者调用；只转发 DB、全仓无接收者/UFCS 消费者的方法不作为“未来接口”保留。
6. legacy 媒体字段只有在 migration、读写双轨和数据回填闭环明确后才能删除；这类数据迁移兼容字段不能与无消费者代码一起机械清理。
7. 仅在 `#[cfg(test)]` 下存在、且只是一对一转发 DB 的 service 不是有效测试夹具；测试应直接验证权威 DB/编排入口，随后删除 test-only service、构造器和错误转换。
8. `host_domain` 只能作为迁移期清单，不应继续吸收应用逻辑；可独立闭环的媒体资产创建、章节后处理等职责应物理迁入对应 orchestrator 域模块。
9. 部署模式环境变量必须严格解析。未设置可使用明确默认值，但设置为非法值不能静默切换模式；例如非法 `A4N_MANAGED_SIDECARS` 现在会阻止启动，而不是悄悄从 managed sidecar 改为 remote。
10. 无真实行为的生命周期 hook 不能以恒定 `Ok(())` 留在生产调用链中。项目完成状态同步 no-op 及其调用、重导出已经删除；后续同类入口必须实现明确状态变更或直接移除。
11. sidecar capability version 必须只来自 `a4n-service-protocol` registry。runtime 请求构造不再散落 `*-v1` 字面量，未知 embedded/disabled service 也必须在 manager 配置期失败，不能伪造 `embedded` capability version。
12. embedding sidecar 协议已升级为 `embedding-v2`：正式支持 `fastembed` 与显式 `local_hash`，拒绝 API backend 回退到 host provider；请求合同删除 host `cache_dir`，并拒绝旧字段与旧 `models` 操作别名，缓存根目录只由 sidecar 环境拥有。
13. sidecar profile 下的提示词运行态必须以 prompt sidecar 为权威来源。全配置导出与提示词列表现在复用同一 sidecar-aware helper，不再从 host 内嵌 engine 导出第二份状态。
14. 跨 crate 文件副作用端口不能返回裸 `String`。项目复制媒体 copier 已使用 `ProjectMediaCopyError` 表达源路径、扩展名、目录创建和文件复制失败，并由 DB/orchestrator 唯一映射。
15. 删除 Lance `create_collection` 恒定成功的无消费者 API 及其全链路路由、client、trait 和测试桩；RAG 集合由首次写入真实记录建立，capability 版本升级为 `rag-index-v2`。
16. sidecar 路径、地址、URL、临时媒体 TTL/大小和媒体 token TTL 的环境变量遵循“未设置才默认；已设置为空或非法立即失败”；prompt sidecar 的旧 `A4N_PROMPTS_DIR` / `A4N_PROMPT_GIT_DIR` 别名已删除。
17. sidecar payload 和 service envelope 统一拒绝未知字段；协议主版本升级为 `2`，prompt/LLM/image/video/import/export capability 版本升级为 `v2`，新旧进程必须在 health/version 阶段显式拒绝。
18. 八个 sidecar binary 的进程启动规则收敛到轻量 `a4n-sidecar-bootstrap`：监听地址与日志过滤表达式只在变量未设置时使用默认值，空值、非 Unicode、非法地址和非法日志表达式均显式失败；各 sidecar 删除对 `tracing-subscriber` 的重复直接依赖。
19. host 与 provider 边界继续执行同一配置规则：server `RUST_LOG`/`A4N_WEB_ADDR`、Tauri API transport/boolean flag、`HF_ENDPOINT`、视频图片 transport、远端临时上传 URL/token 都不能把已配置的非法值静默当作未配置；远端上传 URL 还必须通过 scheme、host、凭证和 fragment 校验。
20. LLM gateway 流事件中的 `UsageInfo`、`ChatChunk` 与 `ServiceTaskEvent` 序列化是明确不变量，不再通过 `.ok()` 生成缺失 usage/payload，或在序列化失败时伪造另一个 task id 的失败事件。
21. 删除无消费者的 `UpdateVideoRecord` 公共兼容别名，并修正公网媒体 URL 对带方括号 IPv6 loopback/unspecified 地址的漏判；完整 video-generation 121 项测试现已全部通过。
22. prompt Git 只有 unborn branch、HEAD 不存在或目标路径不存在可解释为“无历史版本”；仓库损坏、权限和其他 Git 错误必须上抛，不能通过 `Err(_) => None` 隐藏。
23. LLM/image/video 模型发现复用各自主服务 registry capability version；删除 runtime 生产请求中的 `llm-model-discovery-v1`、`image-model-discovery-v1`、`video-model-discovery-v1` 漂移字面量，避免 v2 sidecar 在 envelope 校验阶段拒绝自己的 host 请求。
24. sidecar 响应 payload 与请求同样严格：LLM 嵌套输出、embedding identity/进度、prompt DTO、RAG 结果、image/video/probe/job 输出 wrapper 全部拒绝未知字段；`VideoJobOutput` 对内部标签 unit variant 使用显式反序列化，避免 Serde 对 `pending`/`running` 额外字段的宽容解析。
25. server/Tauri 日志初始化使用 `try_init` 并显式处理 reload handle、日志目录和文件 guard 的 OnceLock 冲突，不再把部分初始化或重复初始化当成成功。
26. 收尾完成声明必须区分代码门禁与环境型集成验收：minimal/full 定向编译、合同测试、API parity、依赖树和 package inventory 通过后，可关闭已知代码残留；Linux 原生重依赖与八项 sidecar 健康链已实跑通过，真实 PostgreSQL、sidecar HTTP 故障注入、跨重启恢复及其他平台的原生依赖/签名安装器仍需在具备对应环境时单独验收，不能用“服务已启动”替代业务 E2E。
27. sidecar bootstrap 必须覆盖所有进程入口，而不是只统一监听地址：prompt、LLM、embedding、RAG、image、video、import、export 以及 demo sidecar 均需先严格初始化各自日志过滤表达式，再绑定地址；已配置非法日志值必须在监听端口前失败。
28. Git 提示词存储的“空仓库”只包括 NotFound/UnbornBranch。仓库打开后的 HEAD 损坏、HEAD/parent/tree/diff 读取错误和 Mutex 中毒必须显式返回 `AppError`，不能重新初始化仓库、返回空历史或把提交当成根提交。
29. 诊断 read model 不能把同步锁失败伪装成空状态：受管 sidecar PID `RwLock` 中毒必须使健康检查显式失败，不能退化为空 PID map 后继续生成误导性的服务健康摘要。
30. 用户可配置枚举值只接受错误信息和文档声明的规范集合；`A4N_VIDEO_IMAGE_INPUT_TRANSPORT` 只保留 `auto`、`inline_data_url` 和 `public_url`。`auto` 统一直接发送图片 data URL；远端临时媒体上传模式、相关环境变量和服务端中继路由已完整删除。
31. sidecar base URL 是路径拼接根地址，必须拒绝查询参数和 fragment；否则后续 `base_url + route` 会产生被忽略或语义错误的 URL。统一 parser 继续拒绝空值、非 HTTP(S)、缺 host 和凭证。
32. 公共 DB/orchestrator API 必须存在真实调用者。无消费者的 `a4n_db::generation::runs_for_workflow` 已删除，workflow detail 保留其唯一实际使用的域内查询，避免同一 generation-run 读取规则存在第二入口。
33. 当前 Linux 环境已实际完成 sidecar 级合同测试矩阵：capabilities 18、embedding service 10、export 3、image 10、import 3、LLM gateway 20 加 5 项独立进程测试、prompt 4、RAG 5、service protocol 41、video 12；八个 sidecar health endpoint 均返回 protocol=2 与对应 v2 capability。该证据覆盖协议/secret/未知字段/跨进程 LLM stream 与 provider 失败隔离，但仍不替代真实 PostgreSQL 业务写入和跨平台安装器验收。
34. 只有一个真实实现且所有消费者都要求存在的 trait 能力不能保留“默认缺失”兼容合同。`PromptEngine::registry` 已从默认返回 `None` 的 `Option` 方法收紧为必需的 registry 返回值；`TeraPromptEngine` 始终持有 registry，runtime 和 prompt sidecar 不再保留理论上不可达的“提示词注册表未加载”分支。
35. 公共文件系统和数据库 handle API 同样按真实消费者收敛。无调用者的 `ProjectPaths::metadata_file`、`ProjectPaths::total_size`、底层 `paths::size` 模块和 `DbHandle::shared_connection` 已删除；文件管理文档与图同步移除这些不存在的合同，避免文档继续把死 API 描述为能力。
36. 批量图片输入的字段优先级不能吞掉高优先级类型错误后继续读取 metadata/default。`batch_image_aspect_ratio` 现在返回 `Result<Option<_>>`，顶层及 metadata/prompt_snapshot/generation_context 中已出现但不是字符串的 `aspect_ratio` 均显式失败，不再通过 `.ok().flatten()` 回退。
37. LanceDB 中每张表都是一个 RAG collection；启动恢复维度缓存时，缺失 `vector` 字段、字段不是 `FixedSizeList` 或维度非正数均视为 collection schema 损坏并阻止启动，不能静默跳过表后返回健康状态。
38. RAG expected-record 预检必须与真实同步使用同一记录构造不变量。大纲、蓝图、角色、场景记录构造失败或外键名称缺失现在在任何写入前显式失败；`rag_ingest_all` 在同步前执行完整 expected-record 预检，避免“预检缩小计数—写入部分有效记录—最终汇总失败”的部分副作用路径。
39. 进度和统计计数不能通过 `unwrap_or(u32::MAX)` 伪造上限。RAG task scope 对超出 u32 的 expected count 显式失败，RAG progress 直接消费已验证的 `scope.total`，过期记录删除数和客户端 expected count 同样使用 checked conversion；图片批量生成复用已验证 scope，场景图片请求数溢出显式失败。
40. Web workflow SSE 序列化失败属于事件合同破坏，不能通过 `if let Ok` 完全丢失。事件投递仍保持旁路 UI 语义，不因无订阅者回滚业务，但序列化失败会记录包含 task/channel/error 的明确错误日志。
41. workflow/checkpoint/order/count 不能在类型转换失败时写入整数上限。视频分阶段 checkpoint、图片/立绘 checkpoint、场景档案同步、画布片段顺序、视频导入镜头号和目录/项目计数均改为范围可证明的无损转换或 checked conversion；越界在任何持久化或 provider 请求前显式失败，生产代码不再存在 `try_from(...).unwrap_or(*::MAX)`。
42. 工作流进度 read model 不能把损坏数据投影成正常默认值。`progress_json` 现在按严格字段、u32 范围、`current <= total`、百分比范围和 `ProgressMeta` 合同解析；持久化事件中的负数、零 total、越界 current、非法 percent/meta 均返回 `WorkflowReadError::InvalidData`，写入端也会在更新 workflow run 前拒绝 u32 到数据库 i32 的截断和 event index 溢出。
43. 业务计数和批量任务进度不能通过饱和运算或未经检查的窄化转换继续运行。优化修复数、清理任务尝试数、孤儿目录总大小、视频批量进度、角色同步批量进度都使用 checked arithmetic；溢出会返回明确错误，不再伪造最大计数、环回进度或截断索引。
44. 测试不能迫使结构化错误退回字符串兼容接口。Project duplicate/freeform remap 和 source-snapshot 测试已直接匹配 `ProjectDuplicateError::{InvalidState, InvalidArgument}` 与 `ProjectMutationError::InvalidArgument`，删除 `.contains()`/`&str` 旧断言；权威错误分类、current/operation 上下文和消息继续由生产枚举唯一持有。
45. 视频规划的业务序号和派生计数不能依赖 `usize as i32`、浮点直接窄化或饱和区间计算。动作节拍、镜头、单镜头节拍、分段编号/索引、已存规划连续编号、required segment count 和 planning-quality 总数均执行 checked conversion/arithmetic；反向 source-span range 和风险阈值溢出显式失败。Video Prompt JSON 81 项已全部通过。
46. 章节和场景正文字符数属于数据库 i32 合同，不能通过 `chars().count() as i32` 截断。章节手工编辑、导入审核、生成章节完成、章节导入、文件解析总字数、优化补丁写回、生成/更新章节场景均在查询或写入前执行 checked conversion；生成场景错误合同补充 `InvalidArgument` 并由 orchestrator 映射，超大正文不会写入负数或环回字数。
47. 导入章节编号和章节总数不能通过 `usize as i32` 写入审核、分析进度或蓝图。TXT parser 的章节拆分改为可失败结果，逐章编号 checked conversion；import-review parsed count、导入分析 chapter count 和导入蓝图 total_chapters 同样在副作用前拒绝超出 i32 的集合。
48. 项目回滚影响是删除前安全提示，不能截断数据库 COUNT 或集合长度。章节/章纲/分部数量、媒体/任务/档案/场景/蓝图各类 u64 count 均 checked 转为 API i32，蓝图四类影响总数 checked 相加，projection 改为可失败合同；回滚影响定向 3 项已通过。
49. 持久化序列号不能在耗尽时饱和并继续覆盖最后一个值。章节场景重排临时序号、Canvas assembly timeline version、workflow cancellation event index 和 optimization turn event seq 均使用 checked arithmetic；相关 DB error 合同补充明确 InvalidArgument/InvalidData 分类，溢出时事务失败而不是写入重复最大序号。
50. RAG 索引的 Arrow/Lance/Tantivy 边界必须拒绝平台范围截断。向量维度 checked 转为 Arrow i32，metadata chapter_number checked 转为 i32；Lance count/delete/completeness 与 Tantivy num_docs checked 转为 usize，delete 后行数增加视为索引状态异常而非通过 `saturating_sub` 返回 0。RAG index 完整 21 项继续通过。
51. RAG 清理不能把非法章节号当成“无需删除”并恒定成功。`delete_chapter_records` 对非正 chapter_number 返回 InvalidArgument，显式转换为 u32；跨 data-type 删除计数 checked 累加，避免非法请求和计数溢出都表现为成功结果。
52. 生产公共 mutation/query API 必须有真实生产消费者，测试不能成为危险旧入口继续存在的理由。视频提示词启动/状态更新已只保留“视频 + prompt + generation run”事务入口；删除 auth username/email 测试查询 facade、非事务 inspiration 单消息写入、重复 snapshot anchor rebuild 和 profile evidence replace，测试改验权威事务/创建/merge 路径。
53. LLM 上下文预算不能用饱和运算把 token 算术溢出解释成“刚好可用”或“无法装入”。输出预留、安全余量、输入利用率、候选包、后续必需包、稳定前缀和必需 suffix 全部使用 checked arithmetic；两项极值测试验证 reserved sum 与 required suffix 溢出显式失败。
54. 只为跨 crate 测试构造内存提示词的 API 不能常驻 release public surface。`TeraPromptEngine::with_defaults` 已由显式 `a4n-prompt/test-support` feature 控制，只在 prompt 自测及 services/runtime dev-dependency 中启用；默认特性 rustdoc 不再导出该构造器。
55. RAG 同步统计不能通过饱和计数掩盖新增、更新、跳过、删除、分块、业务项和 progress 的溢出，也不能把“新增 + 更新 > expected”投影成 0 skipped。`ExpectedSyncStats` 集中执行 checked add/merge，所有 ingest 调用传播错误；client ingest 使用 checked sum/sub 验证统计一致性，7 项纯合同测试通过。
56. 剩余业务序号、累计大小和比例计算必须区分“明确夹取”与“隐藏溢出”。导出章节总进度、PDF 进度、视频/章节 stream 序号、视频轮询次数、inline 参考图总大小、assembly push 批次、章节目标字数比例、混合检索 top_k 扩展均改为 checked 或无损宽类型计算；Tauri 不再维护第二份 stream index。
57. 基础进度和语义分块算法不应靠饱和算术维持表面单调。`ProgressSpan` 拒绝反向区间并用 u128 完成比例映射，structured JSON progress 的 unit 累计升级为 u128 且溢出为明确不变量失败；语义分块累计字符数沿现有 `AppError` 返回；生产 `saturating_add` / `saturating_mul` 扫描清零。
58. `runtime_config_snapshot` 只能有一个线格式。前端和 command API 的 snake_case 已成为唯一合同，Rust DTO 删除全部 camelCase/snake_case `serde(alias)` 双轨并为 snapshot 及四类 provider config 增加 `deny_unknown_fields`；序列化/redaction 同样输出 snake_case，camelCase 与未知嵌套字段测试均显式失败。
59. 饱和减法不能掩盖数据源不一致。Prompt registry 删除数改由 retained-current 事实计算，Git history 在非空前置条件下精确排除最新提交，LLM usage 拒绝 cache hit 大于 prompt tokens，FastEmbed 拒绝下载总量溢出或 downloaded > total，视频批量结果拒绝 failed > total；其余 `saturating_sub` 仅保留明确的 tail、overlap、budget、window 或 timeout 夹取语义。
60. 生产数值边界不能依赖 `as u32` 截断，也不能用零值或范围夹取伪造可用进度。工作流写入、Galgame、大纲、批量导出/视频/角色、RAG 章节范围、字节下载比例和分部章节计数均改为显式验证、checked arithmetic 或宽类型比例映射；非法 `total=0`、`current>total`、反向章节范围与超范围转换在副作用前失败，生产 Rust 源码中的 `as u32` 已清零。

因此，后续新增 facade、fallback 或共享合同 crate 时，除了“能编译”，还必须提供第二个真实消费者、明确失败语义或数据迁移闭环之一；否则视为未完成的重构残留。

完整体系化重构可以认为完成的条件：

1. 日常开发有明确的模块级构建、检查、启动、重启命令。
2. prompt、LLM、embedding、RAG、image、video 都有独立 sidecar binary。
3. 修改任一 sidecar 的实现代码时，只需要编译对应 sidecar 并重启服务，`a4n-tauri` / `a4n-server` 不需要重新编译。
4. host minimal profile 不再编译 RAG/embedding/vector DB/image/video provider 重依赖。
5. `a4n-tauri` 和 `a4n-server` 不再直接依赖 provider crates，只依赖 orchestrator/client/protocol。
6. migration 不再无条件进入所有运行时路径，除非有明确产品要求。
7. sidecar 不可用、版本不兼容、能力 disabled 时都显式失败，不做静默 fallback。
8. runtime config、用户上下文、任务进度、取消、artifact 都通过显式协议跨进程传递，不依赖 task-local 或进程内 token。
9. 数据库写入、权限校验、workflow/checkpoint 仍由 orchestrator 统一控制。
10. schema 变更只通过 `a4n-migration` 管理，runtime 不再维护第二套 schema 修改 SQL。
11. full build/check 仍可作为交付前兜底验证。
12. 文档、脚本帮助、Cargo feature、服务配置和实际运行模式保持一致。
