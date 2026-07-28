# 工作包 05：A3 管理域、读路径和残余小域

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 对剩余 P2/P3 域做分类审计，只迁真实业务流程，确认合法 Host 边界并关闭 3.10 的长尾 |
| V2 来源 | 3.10、阶段 A；进度看板「中小域 / 其它 Host API 域」 |
| 当前状态 | 已收口；全部 AppState actions 已按 Host adapter/IO/DTO/read model 分类，最新观察为 253/829，未知一级域、本地纯 rules 和透传壳均为 0，批次 303 已关闭 V2 3.10/阶段 A |
| 可并行性 | 可拆成 2–3 个子 agent，但必须按目录互斥；不要同时改公共 Host 根模块 |

## 候选队列

先按当前源码重新盘点，下面只是路由清单，不是要求全部迁移：

| 子组 | 候选域 | 重点判断 |
|---|---|---|
| 管理/状态 | `admin_records`、`task_records`、`project_status`、`generation_analytics` | 权限、状态写入、队列统计是否已有 owner；读路径可保留 Host |
| 公共/协作 | `project_collaboration`、`oauth`、`logging`、`storage_paths` | OAuth/JWT/外部 IO 通常是 Host；成员/邀请业务写入应走 application |
| 创作工具 | `manga_records`、`manga_pipeline`、`optimize`、`inspiration_records` | DB owner 可能已完成，但流式 LLM/workflow 是否仍是完整 Runtime 流程 |
| 媒体/模型 | `[~] image_records`、`rag_search`、`embedding_model`、`request_preview` | `image_records` application owner 已收口；文件/provider/sidecar IO 可保留，其他入口继续分类 |
| 项目/档案长尾 | `character_records`、`project_records`、`project_collaboration` | 读模型与文件清理边界；不要重做已完成 CRUD application |

## 当前进度

- `[~] image_records`：项目/章节列表权限、视频参考图范围与排序、导入校验和 metadata、创建/更新/收藏/删除已由 `a4n-orchestrator::media` 单点拥有。
- Runtime 只保留物理文件创建/写入、`persist_created_media_file`、DB 写入失败后的 cleanup outbox 和 DTO 投影；真实 PostgreSQL 集成验证完成前不标记 `[x]`。

## 全量残余分类（2026-07-25）

本轮按一级 Host API 域核对基线 261 个引用 `AppState` 的 actions 文件；基线为 865 次文本命中。当前共享工作区度量为 263/888，增量来自 09 会话新增的 `generation_tasks/destructive_lifecycle.rs` 与 `generation_tasks/mod.rs`（及其生命周期调用点）。分类依据不是文件名，而是当前调用链、Orchestrator application owner、结构门禁和保留的 Host 副作用。基线扫描确认：生产 actions 直接 `.orchestrator()` 为 0，直接 `state.db` 为 0，本地纯 rules 与应删 Orchestrator 透传壳均为 0。

| Runtime 入口组 | 唯一业务 owner | Runtime 保留分类 | 结论 |
|---|---|---|---|
| `admin_records`、`generation_analytics` | `context/admin.rs` / `AdminRuntimeFacade` | 管理员认证边界、读模型和 DTO | `application + read_model`，无需再迁 |
| `auth_records`、`oauth` | `context/auth.rs` / `AuthRuntimeFacade` | JWT、OAuth 跳转/网络、token DTO | `application + host_io`，OAuth/JWT 不迁 |
| `project_collaboration`、`actions/access.rs` | `context/collaboration.rs` | 当前用户权限 Host、错误字符串适配 | `application + host_io`，权限规则不复制 |
| `config_records`、`settings_all`、`theme` | `context/settings.rs` 与 theme application | provider 连接测试、模型发现、runtime config、DTO | CRUD/事务归 application；探测为 `host_io` |
| `project_records`、`project_status` | project read/mutation/status/family applications | 列表/summary、cleanup outbox、文件清理、DTO | 写路径归 application；读模型/补偿 Host 合法 |
| `project_duplication`、`project_transfer` | project duplication/family transfer/video-studio transfer/text import applications | ZIP、archive、媒体磁盘、路径映射、cleanup outbox | `application + host_io`，不再迁物理 IO |
| `character_records`、`character_ai` | `context/character.rs` 与 character applications | Prompt/RAG、进度/取消、文件清理、DTO | 完整业务链归 application；Host Port 合法 |
| `character_implicit_review`、`character_conflict` | `character/{implicit_review,conflict}` | Prompt renderer、进度、任务与 DTO | 批次 216 已收口 |
| `object_profile_records`、`scene_profile_records`、`scene_profile_sync` | asset/scene-profile applications | 下游图片文件、RAG trace/ingest、进度与 DTO | CRUD/同步归 application；媒体/RAG 为 Host |
| `chapter_content`、`chapter_generation`、`chapter_scene`、`outline_records` | chapter-content/chapter-generation/chapter-scene/outline applications | Prompt/embedding、进度/取消、analysis Host Port、DTO | 完整生成与写路径归 application |
| `blueprint_records`、`shooting_script_records`、`source_sync` | project-blueprint/shooting-script/source-sync applications | Prompt、队列、`TaskFamilyContext`、进度与 DTO | 批次 216 已收口 |
| `image_generation`、`image_records`、`portrait_generation`、`portrait_records`、`profile_materials` | media image/record、portrait、profile-material applications | provider/队列、Base64 边界、物理文件、cleanup outbox、DTO | `application + host_io`，文件边界不迁 |
| `video_generation`、`video_records`、`video_prompts` | video provider/records/prompt applications | provider/sidecar、FFmpeg、文件、缓存、队列、进度、DTO | 完整状态机归 application；媒体 transport 为 Host |
| `canvas_workflow`、`galgame_records` | canvas graph/node/media-import 与 galgame applications | provider、文件、FFmpeg、队列、WebGAL 文件 Host Port | graph/pipeline/checkpoint 归 application |
| `manga_records`、`manga_pipeline`、`optimize` | manga/optimize applications | Prompt/LLM transport、进度、媒体文件和 DTO | 完整 workflow 归 application |
| `inspiration_records` | inspiration application | streaming transport、Prompt/进度与 DTO | 消息/上下文/落库归 application；流为 Host |
| `import_analysis`、`import_reverse`、`import_review`、`export_files` | import/export applications | 上传/模板变量、trace、文件/格式输出、进度与 DTO | 完整 workflow/事务归 application |
| `import_file` | `a4n-import-core` canonical parser | Base64/data URL、文件类型选择和结果 DTO | 纯上传 `host_io`，不建第二个 application |
| `rag_search`、`embedding_model` | RAG application + capability contracts | sidecar/model 下载、缓存目录、provider 生命周期 | 检索规则归 owner；下载/缓存为 `host_io` |
| `task_records` | workflow application、持久队列与 canonical `plan_unified_resume` | 进程内 task handle、权限、Canvas/Video Host resume dispatch、DTO | `application + host_io`；不新增 catch-all task application |
| `request_preview`、`dependency_conflict` | canonical request/dependency contracts | 模型预算、Host preview/status DTO 投影 | `dto_adapter`，无需迁 |
| `logging`、`storage_paths` | 无业务状态 owner；读取 project read model | 诊断、路径、孤儿目录扫描 | 明确 `host_io/read_model` |

### 删除与死代码审计

- `AppState` 所有 crate 内用途 getter均有生产消费者；未发现可以直接删除的 getter。
- `state/*.rs` 中所有 `pub(crate)` facade 方法均有消费者；未发现死 facade 方法。
- 已删除的 `GenerationRuntimeFacade` 不恢复；结构门禁改为断言 `state/generation.rs` 与 `AppState::generation` 均不存在。
- 未新增 `ManagerApplication`、`MiscApplication`、catch-all facade、兼容 wrapper 或第二份权限/状态/媒体规则。

### 当前结论

基线范围内剩余 `AppState` 文本命中已全部归入 `application adapter`、`host_io`、`dto_adapter` 或 `read_model`，没有“未知 owner”的一级 Host API 域。当前新增的两个 `generation_tasks` 文件不在本包 owner 范围，必须由 09 会话先完成 lifecycle/Host 边界交接，再更新本包分类快照。工作包 05 的基线代码分类审计已完成；真实 PostgreSQL 用例、跨域媒体补偿和最终 3.10 关闭仍分别由工作包 07、09 和协调者处理，因此本文件不单独宣告 V2 3.10 完成。

验证快照：

```text
当前共享工作区 `scripts/checks/refactor-metrics.sh --label 'wp05 audit (no append)'`：通过；本地纯 rules 0/0、Orchestrator 透传壳 0/0、生产 runtime `.orchestrator()` 0/0；观察量 263/888
此前共享工作区 cargo check -p a4n-orchestrator -p a4n-runtime --no-default-features：通过
此前共享工作区 cargo check -p a4n-server -p a4n-tauri --no-default-features：通过（仅既有 unused-import warning）
批次 216 稳定点 convergence：67/67 通过
最新并行删除后的 convergence 重跑：60 秒内停在测试二进制编译/链接，未进入 runner，不计为通过
A4N_TEST_DATABASE_URL / DATABASE_URL：未配置，PostgreSQL 断言未进入 runner
```

### 并行复核交接（2026-07-25）

- 未修改 09 的 `generation_tasks` 文件，也未把新增观察量伪装成 05 的迁移成果。
- 09 交接后需重新运行 `scripts/checks/refactor-metrics.sh`，并为新增入口补 `application/host_io/dto_adapter/read_model` owner 证据；在此之前 05 保持等待协调者聚合。

## 分类规则

每个入口必须标记为以下一种：

1. `application`：权限、不变量、workflow/checkpoint、任务状态、DB 事务、capability 业务调用或跨步骤失败语义。
2. `host_io`：JWT/OAuth、路径、文件、FFmpeg、网络 transport、窗口/进程、队列句柄或本地 provider 执行。
3. `dto_adapter`：Web/Tauri DTO、错误映射、进度或 preview 投影。
4. `read_model`：无业务状态推进的列表/详情读取；可由窄 facade 保留，不强制迁到新 application。
5. `dead_or_duplicate`：无消费者、第二来源或仅透传，直接删除。

没有分类证据时不得批量搬文件，也不得因为目录仍引用 `AppState` 就判为未迁业务。

## 执行内容

1. 生成 public Host API → owner → DB/capability/file/event 的表。
2. 对 `application` 类入口复用现有 context/application；只有确实无 owner 时才新增最小 owner。
3. 对 `host_io` / `dto_adapter` / `read_model` 写明保留理由，并删除误导性的 rules/application 命名。
4. 删除 dead facade、重复 DTO/规则/错误映射和无消费者公共重导出。
5. 增加结构门禁，阻止已经收口的域重新通过完整 context、裸 DB 或第二套状态机回流。

## 不在范围

- 不创建通用 `ManagerApplication`、`MiscApplication` 或 catch-all facade。
- 不迁移 OAuth、JWT、窗口、进程、文件、FFmpeg 和 transport 本身。
- 不修改公共 API 合同 crate；交给 06。
- 不新建媒体清理、事件桥或 sidecar 测试基础设施；交给 07–09。

## 关闭条件

- 看板中的剩余小域都完成分类，没有“未知 owner”的公开 Host API。
- 真实完整业务流程归 Orchestrator；合法 Host 边界有清晰、可审查的保留理由。
- 无 catch-all abstraction、第二套规则、薄透传壳和死公共 API。
- 3.10 的关闭审计可以逐入口证明 Runtime 不再拥有可独立跑完的业务流程，而不是依赖行数推断。

## 最小验证

```bash
cargo fmt --all -- --check
git diff --check
timeout 60 cargo check -p a4n-orchestrator
timeout 60 cargo check -p a4n-runtime --no-default-features
timeout 60 cargo check -p a4n-server --no-default-features
timeout 60 cargo check -p a4n-tauri --no-default-features
scripts/checks/refactor-metrics.sh --label "A3 residual audit"
```

## 交接要求

必须提交完整分类表，包括“无需迁移”的入口及理由。禁止用“目录已扫描”“AppState 已减少”代替入口级 owner 证据。
