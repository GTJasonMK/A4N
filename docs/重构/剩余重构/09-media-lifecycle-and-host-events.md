# 工作包 09：媒体/文件生命周期与统一 Host 事件桥

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 统一跨事务文件副作用、临时 artifact、cleanup outbox、恢复和 Server/Tauri 事件映射 |
| V2 来源 | 3.15、3.16、阶段 F、阶段 G |
| 当前状态 | `已收口`。普通失败补偿、outbox 幂等、受管媒体与非媒体 orphan project 目录启动恢复、临时 TTL、单 Host data-directory ownership、canonical generation terminal/snapshot TTL、task tree/outbox 保留期和唯一 generation transport均已统一；批次 302 已取得本地 Linux 真实 Tauri/Wry 首帧、重挂载和 Destroyed 清理证据，批次 303 按原始关闭条件完成最终裁决。远端 paid provider/artifact、Actions 和跨平台发布证据转交工作包 10，不再阻塞 3.15/3.16 |
| 可并行性 | 生命周期 inventory 可立即开始；最终 owner/补偿实现需等待 01–05 的业务 owner map |

> 批次 262 增量：Optimize chat 已接入唯一 generation transport/store/reducer/terminal 路径，旧 `send_optimization_message` 生产入口已删除；Docker `a4n_test` 已实际通过 DB/迁移测试，并补齐真实 Server provider 失败终态、SSE 关闭、幂等重试和正式消息不落库证据。工作包仍未整体关闭，成功 Web/Tauri process、其他页面、provider/artifact owner 和 `IG-CUTOVER` 仍是未完成边界。

> 批次 263 增量：Optimize analysis 与 patch proposal 已迁入同一 generation transport。新增 analysis/patch worker 与 `GenerationTerminalCoordinator` 完成事务，正式结果统一为 `result_ref.kind=optimization_turn`、`id=turn_id`、`revision=SHA-256(final_reply)`；analysis/patch/chat 均只消费 `raw_json` generation stream，旧 Optimize progress/stream collector、同步执行入口、Server/Tauri task wrapper 和前端 progress-task 依赖已删除。patch preparation 在 Turn 创建后任一消息/事件写入失败时显式将 Turn 收口为 `failed/cancelled`，不留下 `running` 孤儿。工作包仍未整体关闭，成功 Web/Tauri process、其他页面、provider/artifact owner、持久创建窗口和 `IG-CUTOVER` 仍是未完成边界。

> 批次 270 增量：五种图片生产入口（scene/page/batch/object-profile/scene-profile）已统一迁到既有 canonical generation reservation/session/worker/client/store/reducer/terminal；旧同步生产入口和 `workflow:<task_id>` 图片状态源已删除。批量入口采用 `BestEffort`，稳定 item key 为 `image-request-<1-based-index>`，部分失败保留已成功 artifact；artifact 事件只携带 opaque id、MIME、SHA-256、大小和 revision。Docker `a4n-postgres` 的 `a4n_test` 已实际连接并完成 migrator；当前不要再以“没有测试数据库”解释未执行项。工作包仍未整体关闭，Tauri 成功 process、provider/artifact canonical owner、持久创建窗口、跨进程事件顺序和 `IG-CUTOVER` 仍缺。

> 批次 273 增量：真实 Server HTTP SSE 客户端已对 1 MiB inspiration 回复主动断线 10 次并按 cursor 重连，最终 76 个事件连续且正式结果/hash 一致；共享文本 batching 已从自定义 512 字节收敛到协议 16 KiB 上限。该证据不冒充真实浏览器、外部代理或 Wry；provider cleanup owner、持久创建窗口、跨进程完整顺序和 `IG-CUTOVER` 仍缺。

> 批次 032（增量生成进度记录）当时将 Host 后台通道改名为 `host-task:{task_id}`；批次 276 进一步确认该通道没有前端消费者并将整条桥删除。前端 `WorkflowEvent`/`workflow:${id}`/`createProgressTaskId`、生产 `chat_llm`/`llm_full_response` 和 Host task event 均已归零。批次 277 又关闭本地持久创建强杀窗口证据；本工作包仍未整体关闭——真实浏览器/Wry Destroyed、provider 实际 delete/cancel、远端 artifact 生命周期与跨进程 canonical generation 完整顺序仍缺。

## 子包 A：媒体/文件生命周期（3.15）

1. 对当前约 61 个直接文件副作用入口建立清单：创建 owner、DB owner、补偿 owner、恢复 owner、TTL、是否持久。
2. 复用现有 cleanup outbox；禁止新建第二套后台清理、状态机或 retry 表。
3. 覆盖上传、生成、导入、裁剪、assembly、cache、temporary artifact、remote upload/download 的统一 created/deleted 合同。
4. 持久文件先有可恢复意图或原子清单；删除必须幂等、可重试、可跨重启恢复。
5. 远程临时 artifact 只返回 artifact id、HTTPS URL、受限 `expires_at` 和 owner/audit 信息；Host 不信任任意本地路径。
6. 以真实 PostgreSQL 集成测试覆盖 DB 成功/文件失败、DB 失败/文件已写、删除失败、worker 重启和重复 claim。

## 子包 B：统一 Host 事件（3.16）

1. Orchestrator/TaskHandle 产生 canonical task/workflow event；Runtime 只做边界适配。
2. Generation 只使用专用 Server SSE 与 Tauri Channel；普通后台任务没有消费者时只保留可查询快照和取消，不创建第二份事件 payload。
3. 保持 sequence、usage、checkpoint、artifact、cancel ack 和唯一终态顺序一致。
4. 未注册任务不得通过通用字符串 channel 伪造 fallback；需要实时事件时必须先定义真实消费者和 canonical 合同。
5. process test 直接断言 canonical generation 的 Server SSE/Tauri Channel 顺序、终态唯一性和取消确认。

## 依赖与边界

- 01–05 提供每个业务域的文件副作用 owner；本包提供跨域生命周期基础设施，不替代业务 application。
- 07 提供 PostgreSQL/process runner；本包补充媒体失败组合测试，不修改通用 CI 入口。
- 08 提供 remote auth/egress policy；本包只实现 artifact owner/TTL/audit 接线。
- 10 负责 metrics/logging；本包只发 canonical lifecycle/event 数据。

## 主要路径（先核对）

```text
crates/a4n-db/src/cleanup_outbox.rs / crates/a4n-db/src/**
crates/a4n-orchestrator/src/context/media.rs
crates/a4n-orchestrator/src/**/*cleanup*
crates/a4n-runtime/src/actions/**      # 只按清单逐项处理
crates/a4n-runtime/src/state/**
crates/a4n-runtime/src/host_api/**
crates/a4n-server/src/**event* / crates/a4n-tauri/src/**event*
crates/a4n-core/src/event/**
```

## 关闭条件

- 所有持久文件副作用都有唯一 owner、TTL、补偿和恢复入口。
- 删除/上传/生成/导入/assembly/cache 的失败组合可重试且幂等，DB 与文件没有两个无法协调的事实源。
- worker 重启、强杀、重复 claim 和过期 artifact 有真实测试。
- Runtime 业务 action 不拼接 Web/Tauri 特定事件；Server/Tauri 使用同一领域事件输入。
- process test 证明顺序、usage/checkpoint/artifact/cancel ack 和唯一终态一致。

## 最小验证

```bash
cargo fmt --all -- --check
git diff --check
timeout 60 cargo check -p a4n-db
timeout 60 cargo check -p a4n-orchestrator
timeout 60 cargo check -p a4n-runtime --no-default-features
timeout 60 cargo test -p a4n-db cleanup_outbox -- --nocapture
timeout 60 cargo test -p a4n-orchestrator --lib media::
timeout 60 cargo test -p a4n-runtime --no-default-features event
```

没有 PostgreSQL 时只能记录编译和纯单测；DB/文件组合与重启证据必须在 CI 或配置好的集成环境真实执行。

## 交接要求

提交 61 个入口的 lifecycle matrix 和事件序列图；每个未覆盖组合标 `未执行`，不要以“已有 outbox”代替全量关闭证据。

## 2026-07-25 本会话执行记录

### 当前基线

旧计划中的“61 个入口”已经不是当前源码事实。按生产代码重新扫描：

```bash
rg -l --glob '!**/tests.rs' --glob '!**/tests/**' \
  '(std::fs|tokio::fs|File::create|OpenOptions|remove_file|remove_dir|rename\(|copy\(|write\(|write_all)' \
  crates/a4n-runtime/src/actions | sort
```

结果为：

- 31 个包含原始文件 I/O 的生产文件。
- 再加 2 个调用共享 `write_new_data_file` 的项目包导入 Host port。
- 合计 33 个当前生命周期入口文件。
- 生产 action 中的直接物理删除只剩上传临时文件、FFmpeg/缓存部分输出；持久 artifact 删除由 cleanup outbox 执行。
- `AppState` 当前为 258 个生产 action 文件 / 873 次命中；它是 Host 边界观察量，不是本包待机械清零清单。

### 已完成的根因修复

1. `media_workflow_v2_reset` 在同一数据库事务中删除媒体记录并插入 `file_artifact_cleanup` intent。reset worker 不再先删 DB、再直接删文件，避免重试时丢失第一次计算出的路径。
2. cleanup intent 的 dedupe key 现在真正幂等：重复相同合同返回已有记录；相同 key 绑定不同 kind/project/payload 时显式失败。
3. project delete 与 file artifact cleanup payload 在插入和 claim 两侧都校验项目目录边界，拒绝跨项目路径与 `..`；worker 真正删除前再次查询 canonical DB 媒体引用，只删除当前仍未引用的路径。
4. 项目族包和 Video Studio 包导入复用 `write_new_data_file`：`create_new(true)`，不覆盖已有文件，写失败删除本次 partial file。
5. 项目复制在复制动作前登记目标路径，普通复制失败也会进入既有 rollback outbox；provider 视频直接落盘失败复用现有 partial-file cleanup。
6. `TaskHandle` 记录第一个 terminal，terminal 后拒绝普通进度和字节进度；Server/Tauri 发送 `TaskHandle` 返回的同一个 Host 事件。
7. Server progress reporter 删除逐事件 `tokio::spawn`；`WebEventHub` 改为同步 publish，调用顺序即发送顺序。
8. embedding 下载进度 worker 在 terminal 前排空并 join；取消后排队中的迟到进度被 `TaskHandle` 拒绝。
9. Server/Tauri 的已注册任务 complete/error/cancel 由 `TaskHandle` 产生唯一终态；local embedding 取消命令只触发 token，由执行路径发送一次终态。
10. `AppState` 持有 `.a4n-host.lock` 直到 Host 生命周期结束，明确同一 data directory 的单写 Host 边界；不新增 artifact 级 lease。
11. 启动阶段复用既有 scanner、project family facade 和 cleanup outbox，自动恢复无 DB owner 的整项目目录，同时保护仍有项目记录、未完成 cleanup 或 canonical media reference 的目录。

### 生命周期矩阵

| # | 入口 | 创建 / 文件 owner | DB owner | 普通失败补偿 | 重启恢复 / TTL | 结论 |
|---:|---|---|---|---|---|---|
| 1 | `canvas_workflow/media_import/audio/import.rs` | `RuntimeCanvasAudioImportHostPort` + `persist_created_media_file` | Orchestrator `CanvasAudioImport` application / media asset | application 失败调用 `remove_canvas_audio`，写入 file cleanup intent | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 2 | `canvas_workflow/media_import/audio/source.rs` | 仅解析上传/本地来源并向 #1 的受管输出写入 | 同 #1 | partial file 由 #1 共享 helper 删除 | 同 #1 | helper，无第二生命周期 |
| 3 | `character_ai/sync_trace.rs` | Runtime diagnostics trace writer | 无 DB owner | 写失败显式 warning，不影响业务结果 | 无删除 TTL；转交工作包 10 的日志治理 | 非媒体诊断文件；保留策略未关闭 |
| 4 | `galgame_records/media/audio.rs` | `RuntimeGalgameAudioHostPort` + `persist_created_media_file` | Orchestrator Galgame audio application | DB/绑定失败写入 file cleanup intent | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 5 | `galgame_records/webgal_export/api.rs` | 只读受管媒体，组包由 export owner 完成 | Orchestrator export/Galgame | 不创建本地 artifact | 不适用 | 只读边界 |
| 6 | `image_generation/generation_io/files.rs` | image Host port 的唯一生成图片落盘 helper，文件名 `create_new` | Orchestrator image generation application | application 失败调用 Host remove，进入 outbox | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 7 | `image_generation/host_port.rs` | 读取参考图并调用 #6；只承载 provider/文件 Host IO | Orchestrator image generation application | `remove_generated_image_file` 写入 outbox | cleanup worker | Host port 已收口，无第二 DB owner |
| 8 | `image_records/upload/file.rs` | 导入图片 `create_new` helper | Orchestrator media/imported-image application | partial write 由共享 helper 删除 | 持久文件无 TTL | helper；DB 失败见 #9 |
| 9 | `image_records/upload/record.rs` | 调用 #8 后提交相对路径 | Orchestrator imported-image DB owner | DB 失败写入 file cleanup intent，cleanup 写入失败附加到主错误 | 启动受管媒体 orphan scan + cleanup worker | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 10 | `logging/diagnostics.rs` | 只读 daily log | 无业务 DB owner | 不创建 artifact | daily rolling 只分文件，不提供删除 TTL | 只读；日志保留由工作包 10 处理 |
| 11 | `portrait_generation/host_port.rs` | portrait Host port + `persist_created_media_file`，文件名 `create_new` | Orchestrator portrait application | application 失败调用 `remove_portrait_file`，进入 outbox | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 12 | `profile_materials/upload/file.rs` | profile material 上传 helper，文件名 `create_new` | Orchestrator profile material application | `upload/api.rs` 的 Host remove 写入 outbox | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 13 | `project_duplication/media.rs` | duplication Host port 复制新项目媒体；复制前登记目标路径 | DB duplicate transaction / Orchestrator duplication application | 任一复制/DB 失败统一将已登记目标写入 outbox | outbox + 启动受管媒体 orphan scan + orphan project directory recovery | 普通失败与媒体/非媒体目录强杀恢复已接入；真实 process evidence 未执行 |
| 14 | `project_transfer/family_package/package_io.rs` | 导出时只读 canonical media bytes | Orchestrator project family export | 不创建本地 artifact | 不适用 | 只读边界 |
| 15 | `project_transfer/family_package/host_port.rs` | `write_new_data_file`，绝不覆盖已有文件 | DB family import transaction / Orchestrator family transfer | validation/DB 失败将 `written_paths` 写入 outbox | outbox + 启动受管媒体 orphan scan + orphan project directory recovery | 普通失败与媒体/非媒体目录强杀恢复已接入；真实 process evidence 未执行 |
| 16 | `project_transfer/video_studio_package/host_port.rs` | 同 #15 | DB Canvas package import transaction / Orchestrator Video Studio transfer | 同 #15 | 同 #15 | 普通失败与媒体/非媒体目录强杀恢复已接入；真实 process evidence 未执行 |
| 17 | `storage_paths/scanning.rs` | 只读 `projects/<id>`，不跟随符号链接；受管 media roots 扫描普通文件 | project read owner / canonical media reference owner | 媒体候选写入既有 cleanup outbox，worker 删除前再次复核 | 受管媒体与无 DB owner 的整项目目录均在启动自动恢复；带 incomplete cleanup 的项目受保护 | 可发现并自动恢复受管媒体与非媒体 orphan project 目录 |
| 18 | `video_generation/media/provider_images/input.rs` | 只读参考图；按配置选择 public URL、remote temp 或 inline | generated image read owner | 本地不创建文件 | 不适用 | 只读/传输边界 |
| 19 | `video_generation/media/provider_images/remote_upload.rs` | 远端临时媒体服务是 artifact owner | 远端服务；Host 只传 user/project/source owner | HTTP/合同失败显式返回；Host 不信任任意本地路径 | 强制 HTTPS；`expires_at` 不超过 24 小时；记录 artifact/owner/audit 日志 | 批次 267 已将 Runtime/Server 合同收敛到唯一 API DTO，并通过真实上传/读取 process；远端删除证据依赖服务 TTL |
| 20 | `video_generation/media/tail_frames/capture.rs` | FFmpeg tail-frame Host IO | Orchestrator/DB generated image owner | capture/DB 失败将相对路径写入 outbox | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 21 | `video_generation/media/video_files/files.rs` | provider/canvas 视频落盘；写失败删除 partial output | Orchestrator video provider/application owner | 后续 DB/finalize 失败由既有 Host remove 写入 outbox | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 22 | `video_generation/media/video_files/paths.rs` | 只创建受管目录和 canonical 目标路径 | 无独立 DB owner | 无独立状态机 | 随父 artifact | 路径 helper，无第二生命周期 |
| 23 | `video_records/assembly/actions/preview.rs` | assembly preview cache owner | DB 只提供 clip/timeline 输入，cache 非 canonical DB 状态 | 失败即时删 output/manifest；失败时再写 outbox | 启动扫描 + cleanup worker；TTL 7 天 | 临时 cache 已收口 |
| 24 | `video_records/assembly/cache/expiry.rs` | assembly cache expiry scanner | 无独立 DB owner | 过期文件写入既有 outbox | 启动恢复；TTL 7 天 | 临时 cache 已收口 |
| 25 | `video_records/assembly/cache/manifest.rs` | 只读 clip metadata/audio probe，生成 manifest value | 无独立 DB owner | 不直接创建 artifact | 随 #23/#24 | 只读/helper |
| 26 | `video_records/assembly/cache/storage.rs` | cache manifest 临时文件 + rename | 无独立 DB owner | 写/rename 失败删除 `.tmp`，父流程再清 output/manifest | #24 启动恢复；TTL 7 天 | 临时 cache 已收口；并发同 key 仍由上层单任务约束 |
| 27 | `video_records/imports/contracts.rs` | `ImportedVideoSource` Drop 删除上传临时文件 | 无 DB owner | Drop 幂等删除，失败 warning | 启动扫描；TTL 24 小时 | 临时上传已收口 |
| 28 | `video_records/imports/processing/canvas.rs` | FFmpeg 抽帧输出 | Orchestrator video records / Canvas import application | caller 在 DB 失败时写入 outbox | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 29 | `video_records/imports/processing/trim.rs` | FFmpeg trim 输出 + partial cleanup | Orchestrator video records application | 创建/完成状态失败写入 outbox并显式记录 failed | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 30 | `video_records/imports/source/copying.rs` | 导入视频复制 helper + partial cleanup | Orchestrator video records application | copy 失败删除 pending DB record；后续失败写入 outbox | 启动受管媒体 orphan scan + cleanup worker；持久文件无 TTL | 普通失败与启动强杀恢复已接入；真实 process evidence 未执行 |
| 31 | `video_records/imports/source/paths.rs` | 只校验来源并创建受管目标路径 | 无独立 DB owner | 无独立状态机 | 随父 artifact | 路径/helper |
| 32 | `video_records/imports/source/upload.rs` | `temp_dir/video_uploads/video_upload_*`，文件名 `create_new` | 无 DB owner | partial write 删除；Drop 删除 | 启动扫描；TTL 24 小时；跳过非普通文件/符号链接 | 临时上传已收口 |
| 33 | `video_records/playback.rs` | 只读受管视频，限制内联 256MB | generated video read owner | 不创建 artifact | 不适用 | 只读边界 |

### 生命周期关闭边界

1. 受管媒体目录现在在启动阶段扫描普通文件，先由 canonical DB media reference 过滤，再写入既有 cleanup outbox；worker 删除前再次复核。这个恢复只覆盖 `images/videos/portraits/audio/media/assets`，不把任意 data 目录当成媒体。
2. 新项目复制/包导入产生的非媒体项目文件现在由同一 orphan project directory scanner 在启动阶段自动回收；管理员命令仍保留为按需手工触发入口。只删除无 DB project、无 incomplete cleanup 且无 canonical 媒体引用的项目目录，不扩大为无边界递归删除。
3. 不新增第四套 manifest、created-file 表或第二个 worker。后续新增受管媒体根目录必须接入同一 scanner 和 canonical reference 查询。
4. character sync trace 与 daily log 没有删除 TTL；属于工作包 10 的日志/可观测性保留策略，不在本包复制一套清理机制。
5. 当前生命周期合同是单 data directory 单写 Host；只有产品明确支持共享写存储时，才重新评估 artifact 级数据库 lease。

### Host 事件边界矩阵

| 路径 | canonical owner | Host 映射 | 终态/顺序 |
|---|---|---|---|
| Canonical generation | `GenerationTaskEvent` + durable snapshot/replay | 专用 Server `/generation-tasks/{task_id}/events`；Tauri IPC Channel | sequence、usage、checkpoint、artifact、cancel ack 和唯一 terminal 由同一 generation owner 保证 |
| Server/Tauri 已注册后台任务 | `TaskHandle` | 无 push 映射；只提供进度快照、有限历史和取消 token | 执行结束立即从 `TaskManager` 移除，不保存第二份 terminal 状态 |
| Server/Tauri embedding 下载 | `TaskHandle::update_progress_bytes` | 原始 byte 仅用于换算并写任务快照；前端由调用 Promise/本地 control 呈现 | cancel 只触发 token，执行路径返回明确 cancelled/error |
| Manga workflow | 持久 workflow status | 前端轮询 `getMangaPipelineStatus` | 不再创建通用 Host progress/complete/error channel |
| `chapter-analysis-updated` | Tauri 章节命令成功回调 | 仅原生 Tauri domain notification | Web 无生产者，因此不伪造通用 SSE；Web 页面依靠命令结果和 focus refresh |
| Tauri local embedding | local runtime cancellation map | 无事件桥；调用 Promise 返回最终结果 | cancel token 是唯一运行中控制，任务结束即移除 |

```text
generation producer -> durable GenerationTaskEvent/snapshot -> Server SSE / Tauri Channel -> generation client
background worker   -> TaskHandle progress snapshot/history -> query/cancel API
background result   -> command Promise/result -> local UI control；TaskHandle 随即移除
```

### 事件关闭边界

1. `ServiceTaskEvent`、`SseEvent`、`HostTaskEvent`、`EventBridge`、`WebEventHub` 和通用 `/events/{channel}` 已删除；不得以改名、alias、通用 JSON channel 或无消费者 fallback 复活。
2. `a4n-service-protocol`/`a4n-api-contract` 的 `GenerationTaskEvent` 是唯一 generation event；后台任务只保留查询/取消状态，除非先出现明确产品消费者和独立合同。
3. 本地真实 Server/Chromium 已取得 usage、durable checkpoint、artifact refs、cancel ack 单 process 组合；真实 Linux Tauri/Wry 已取得活动 cursor 重挂载、回复唯一性与 `WindowEvent::Destroyed` 精确清理。Windows/macOS CI 属于跨平台发布工作包 10。

### 待统一验证

代码集中完成后一次执行：

```bash
git diff --check
timeout 60 cargo check -p a4n-core
timeout 60 cargo check -p a4n-db
timeout 60 cargo check -p a4n-orchestrator
timeout 60 cargo check -p a4n-runtime --no-default-features
timeout 60 cargo check -p a4n-server
timeout 60 cargo check -p a4n-tauri
timeout 60 cargo test -p a4n-core event::channel -- --nocapture
timeout 60 cargo test -p a4n-runtime --no-default-features file_utils -- --nocapture
timeout 60 cargo test -p a4n-db cleanup_outbox -- --nocapture
timeout 60 cargo test -p a4n-db media_workflow_reset -- --nocapture
timeout 60 cargo test -p a4n-server --test cleanup_recovery -- --nocapture
```

没有 `A4N_TEST_DATABASE_URL` 时，PostgreSQL 用例只能记录为“未进入 runner”，不得记为通过。

### 2026-07-25 实际验证结果

- `cargo check -p a4n-core`、`a4n-db`、`a4n-orchestrator`、`a4n-runtime --no-default-features`、`a4n-server`、`a4n-tauri`：通过。
- `cargo test -p a4n-core event::channel -- --nocapture`：8 passed。
- `cargo test -p a4n-runtime --no-default-features file_utils --lib -- --nocapture`：29 passed。
- `cargo test -p a4n-server service_and_server_preserve_progress_terminal_order -- --nocapture`：1 passed。
- `cargo test -p a4n-tauri service_and_tauri_preserve_progress_terminal_order -- --nocapture`：1 passed。
- `cargo test -p a4n-migration migrations_keep_user_default_ai_config_history_marker -- --nocapture`：1 passed；四个历史 migration 已重新注册并保持末尾执行顺序。
- `git diff --check` 与本包涉及文件的定向 `rustfmt --check`：通过。
- `scripts/checks/check-refactor-docs.sh`、`scripts/checks/check-task-kind-boundary.sh`：通过。
- `scripts/checks/refactor-metrics.sh` 最终收口：本地纯 rules **0 文件 / 0 行**、orch 透传壳 **0 文件 / 0 行**；`AppState` 观测值 **263 文件 / 888 命中**。
- 删除两个无消费者透传壳并启用 workspace `reqwest/native-tls` 后，`cargo check -p a4n-runtime --no-default-features` 与 `cargo check -p a4n-server -p a4n-tauri --no-default-features`：通过。
- `cargo test -p a4n-db cleanup_outbox`、`media_workflow_reset`：未执行；当前未配置 `A4N_TEST_DATABASE_URL` / `DATABASE_URL`，不能伪记为通过。

## 批次 226 · cleanup drain 单条失败不再饥饿后续任务 · 2026-07-25

本轮只修复 cleanup worker 的调度根因，复用现有 outbox lease/retry 状态机，不新增 worker、队列或 fallback：

- `drain_project_cleanup_for` 处理某一条已 claim 任务时，如果租约/数据库边界错误导致 processor 返回 `Err`，现在记录首个错误并继续处理本轮剩余 claim；此前 `?` 会立即退出，使后续 outbox 只能等下一轮甚至等租约过期。
- 函数仍在 drain 结束后返回首个错误，显式项目删除调用和启动/后台 worker 的错误可见性不变；普通文件、媒体 reset、项目删除失败仍由既有 processor 写回 retry 状态。
- 日志带 cleanup id 和 kind，便于定位 lease fencing 或数据库瞬态失败；没有吞错或伪成功。

验证结果：`cargo check -p a4n-runtime --no-default-features` 通过，触及文件 `rustfmt --check` 与 `git diff --check` 通过；迁移历史注册单测 1/1 通过。runtime cleanup 定向测试在 60 秒预算内仍停留在聚合编译，未取得 runner 结果；本机没有 PostgreSQL 配置，DB/process 组合仍标记“未执行”。

## 批次 227 · file artifact 删除前复核 canonical DB 引用 · 2026-07-25

本轮继续复用既有 cleanup outbox、`filter_unreferenced_media_paths` 和 lease heartbeat，没有新增引用表、scanner 或状态机：

- `process_claimed_file_artifact_cleanup` 在物理删除前先通过 Orchestrator 唯一 DB owner 查询当前仍未引用的路径，只把未引用子集交给文件删除 helper。
- 如果 DB 引用复核失败，现有 retry/backoff 会保留 intent；不会把查询失败当成“无引用”并继续删除。
- 这同时保护“业务 DB 已提交，但 cleanup intent 完成更新失败”的恢复场景：后续 claim 会看到文件已有持久化引用并跳过删除，再正常完成 intent。

仍未关闭：文件创建前没有 durable intent 的强杀窗口、真实 PostgreSQL 的 DB/文件组合及 worker 重启 process 证据。

验证结果：`cargo check -p a4n-runtime --no-default-features` 通过，触及 cleanup worker 文件的 `rustfmt --check` 与全树 `git diff --check` 通过。runtime cleanup 定向测试在 60 秒内停留在测试二进制编译/链接，未进入 runner；当前未配置 `A4N_TEST_DATABASE_URL` / `DATABASE_URL`，PostgreSQL 引用与恢复组合仍记为“未执行”。

## 批次 228 · project delete cleanup 路径合同收敛 · 2026-07-25

本轮复用 file artifact cleanup 已有的项目路径边界校验，收紧同一个 DB outbox 合同：

- `ProjectDeletionCleanupPayload` 在插入前和 claim 解析时都要求路径位于 `projects/{project_id}/`，并拒绝 `..`、绝对路径和空项目目录路径。
- 项目删除与 file artifact 两种 payload 共用路径归一化/校验 helper；没有新增 worker、表、扫描器或 runtime fallback。
- 非法历史 payload 会沿既有 claim `fail` 路径显式标记失败，不会把任意数据目录路径交给物理删除 helper。

验证结果：`cargo check -p a4n-db`、触及 cleanup outbox 文件的 `rustfmt --check` 和 `git diff --check` 通过；纯合同单测 `project_deletion_payload_rejects_cross_project_and_traversal_paths` 1/1 通过。PostgreSQL cleanup/删除事务组合仍未执行（当前未配置 `A4N_TEST_DATABASE_URL` / `DATABASE_URL`）。强杀创建窗口、worker 重启和 Host service 字段投影仍未关闭。

## 批次 229 · cleanup kind 公平轮转与跨平台路径合同 · 2026-07-25

本轮继续复用同一个 cleanup worker 和 outbox 状态机，只修复两个边界根因：

- `drain_project_cleanup_for` 在 file artifact、media workflow reset、project delete 三类任务之间轮转 claim；持续到来的 file artifact 不再长期压住后两类任务，`limit` 仍表示本轮最多处理的 claim 数量。
- 单条 processor 失败仍沿批次 226 的规则记录首个错误并继续 drain；claim 数据库错误仍立即显式返回，没有新增队列、worker 或隐藏 fallback。
- DB payload 合同与 Runtime 入队边界都拒绝反斜杠路径，避免 Unix 将 Windows 分隔符或 `..\\` 视为普通文件名后绕过项目路径校验。

验证结果：`cargo check -p a4n-db -p a4n-runtime --no-default-features` 通过；DB 路径合同纯单测 2/2、Runtime cleanup 定向单测 5/5 通过；定向 `rustfmt` 与 `git diff --check` 通过。Runtime 单测首次 60 秒只完成编译、未进入 runner，缓存后重跑取得上述 5/5 结果。真实 PostgreSQL 的重复 claim、worker 重启和 DB/文件失败组合仍未执行；强杀创建窗口与 Host service 完整字段投影仍未关闭。

## 批次 230 · 启动恢复受管媒体 orphan · 2026-07-25

本轮没有新增 manifest、表或 worker，直接扩展现有 `storage_paths` scanner 和 `file_artifact_cleanup` outbox：

- AppState 在对外返回前扫描每个项目的 `images`、`videos`、`portraits`、`audio`、`media`、`assets` 根目录，只收集普通文件，不跟随符号链接；完整 assembly cache 成对文件交给既有 TTL scanner，破损/不成对输出仍可被恢复。
- 扫描结果先调用 canonical `unreferenced_media_paths` 过滤，只有 DB 未引用路径才写入既有 file-artifact cleanup intent；worker 物理删除前仍会再次查询引用，避免扫描与业务提交之间的竞态误删。
- intent token 绑定路径、文件大小和修改时间的 SHA-256 版本，同一文件版本重复启动保持幂等；非媒体项目文件仍不自动删除，保留人工 orphan-project 命令作为唯一 owner。

验证结果：`cargo check -p a4n-runtime --no-default-features` 通过；`cargo test -p a4n-runtime --no-default-features storage_paths --lib -- --nocapture` 4/4 通过；触及文件 `rustfmt` 与 `git diff --check` 通过。当前没有 `A4N_TEST_DATABASE_URL` / `DATABASE_URL`，未执行 PostgreSQL DB/文件/重启组合；多进程共享同一 data directory 也未取得 process evidence。

## 批次 231 · cleanup 真实 Server 进程重启 runner · 2026-07-25

本轮没有新增测试框架、worker、表或状态机；直接用标准库启动现有 `a4n-server` 二进制，并复用 `a4n_migration::test_support` 的独立 PostgreSQL schema：

- 第一个 Server 进程在监听端口前完成启动恢复：删除无 intent 的受管媒体 orphan，完成重启前已经存在的 file-artifact intent，并把“目标实际是目录”的物理删除失败写回同一 outbox 的 pending/error/attempt 状态。
- 测试随后强制停止第一个进程，把失败目标改为普通文件并将既有 retry 置为到期，再启动第二个 Server 进程；第二次启动必须以 attempt 2 删除文件并完成原记录，证明失败状态可跨真实 Host 进程恢复。
- 端口开始监听是唯一同步点，因为 Server 只有在 AppState 启动扫描、outbox drain 和管理员校验完成后才 bind；没有增加测试专用 API、sleep 型假成功或第二套 process manager。
- 子进程沿 SQLx 既有 `options[search_path]` 使用测试 helper 创建的隔离 schema，不会落到公共 schema。

验证结果：`cargo check -p a4n-server --tests` 通过（1m27s），`git diff --check` 通过；当前 `A4N_TEST_DATABASE_URL` / `DATABASE_URL` 均未设置，因此 `cargo test -p a4n-server --test cleanup_recovery -- --nocapture` 未执行，不能把 process/数据库行为记为通过。该 runner 执行后可覆盖受管媒体 orphan、已有 intent 启动恢复、物理删除失败、进程停止及重启重试；DB 写失败后的业务 Host 创建路径注入和完整 service event 字段投影仍未关闭。

## 批次 232 · 真实上传双故障恢复 runner 与事件合同去重 · 2026-07-25

本轮先删除一条不应存在的迁移期旁路，再复用批次 231 的同一个双进程 runner 补真实业务故障，不新增 DTO、worker、状态机或 process manager：

- 撤掉图片/视频链中 `ServiceTaskEvent -> serde_json::Value/SseEvent -> Server/Tauri callback` 的过渡投影及其 reporter/channel。依据 `全量增量生成重构执行计划.md` 的 10.1、D01 和 `IG-CUTOVER`：规范生成事件唯一 owner 是 `a4n-service-protocol`，旧 `ServiceTaskEvent`、`SseEvent` 必须在同一 cutover 整体删除，禁止兼容投影、type alias 或第三来源。本包继续保留现有字符串 progress 与 Host canonical terminal，不提前复制最终合同。
- `cleanup_recovery.rs` 直接调用真实 `/api/auth/login` 和 `/api/commands/upload_image`：第一台 Server 启动后，用 PostgreSQL trigger 让 `generated_image` 写入失败，并让同一失败路径的 `file_artifact_cleanup` intent 写入也失败；断言 HTTP 同时报告主错误和清理错误、真实 `imported_image_*.png` 已落盘、DB 中没有图片记录或 cleanup intent。
- 强制停止第一台 Server、移除故障 trigger，再启动第二台 Server；启动 orphan scanner 必须为该文件写入既有 cleanup outbox，并由同一个 startup drain 删除文件、把 intent 完成为 attempt 1。端口监听仍是唯一同步点，没有固定 sleep 或测试专用 API。
- `a4n-server` 仅增加 workspace 已有的 `reqwest` dev dependency；独立 schema、子进程启动/停止和监听同步全部复用批次 231 的现有 helper。

验证结果：触及 Rust 文件的定向 `rustfmt` 与 `git diff --check` 通过；`cargo check -p a4n-server --tests` 通过。当前 `A4N_TEST_DATABASE_URL` / `DATABASE_URL` 均未设置，因此新增真实 PostgreSQL/process 场景只完成编译，未进入 runner，不能记为实跑通过。

仍未关闭：多进程共享同一 data directory 的写前 lease、非媒体项目文件自动恢复、日志保留策略，以及 `IG-CUTOVER` 的规范生成事件整体替换与 usage/checkpoint/artifact/cancel-ack 真实跨进程证据。工作包 09 和总体 16/44 状态不变。

## 批次 233 · data directory ownership 与非媒体孤儿目录自动恢复 · 2026-07-25

本轮继续走最小 owner 收口：不新增依赖、lease 表、manifest、worker 或状态机；同时复核并清除了并行写回到 `sidecar_clients/response.rs` 的残余 service-event 投影。

- `AppState` 在数据库连接、sidecar 启动和启动恢复前取得 Rust 标准库 `File::try_lock` 的 `.a4n-host.lock`；同一 data directory 的第二个 Host 明确失败，进程强杀/退出后由操作系统释放锁，后续 Host 可重新取得。锁由 `AppState` 持有至进程生命周期结束，保留 `ponytail:` 注释说明未来共享写存储才升级 artifact 级 lease。
- 复用现有 `orphan_project_data_dirs` scanner、`remove_data_dir_in_data_dir` 删除 helper 和 cleanup outbox owner；启动顺序固定为受管媒体扫描 → startup cleanup outbox drain → 整个孤儿项目目录恢复 → 既有 cleanup worker。DB 中仍有 `pending/running/failed` cleanup 的项目目录受保护，避免绕过其 outbox owner；项目记录重新出现或仍有 canonical 媒体引用时拒绝删除。
- Server 读取既有 `A4N_DATA_DIR` 环境变量，process runner 的锁、媒体和非媒体目录断言因此落在同一隔离目录，不再误用默认目录。
- `cleanup_recovery.rs` 增加同目录第二 Host 拒绝、强杀后第三 Host 重取得锁，以及 `projects/<orphan>/chapters/orphan.md` 启动自动删除证据；已有失败 cleanup 目录仍保留到 outbox 重试完成，证明保护条件没有被整目录恢复绕过。

验证结果：`cargo check -p a4n-server --tests` 通过（首次冷编译 29.47s，清理残余投影后热编译 2.84s）；最终 `timeout 90 cargo check -p a4n-runtime -p a4n-server -p a4n-tauri --no-default-features` 通过（9.26s）；本包触及文件的定向 `rustfmt --check`、`git diff --check` 和 Host service-event 投影搜索通过。`cargo test -p a4n-runtime --no-default-features data_dir_lock_rejects_second_owner_and_releases_on_drop --lib -- --nocapture`：1/1 通过（约 5.34s 编译，测试本身 0.00s）；当前 `A4N_TEST_DATABASE_URL` / `DATABASE_URL` 均未设置，PostgreSQL/process runner 仍未执行。另：workspace 全量 `cargo fmt --all -- --check` 仍被其他并行会话的既有格式差异拦截，本包定向格式检查不受影响。

仍未关闭：PostgreSQL/process runner 实跑、日志保留策略、`IG-CUTOVER` 规范生成事件整体替换后的 usage/checkpoint/artifact/cancel-ack 跨进程证据。当前合同是单 data directory 单写 Host；只有产品明确支持共享写存储时，才重新评估 artifact 级 lease。工作包 09 和总体 16/44 状态不变。

## 批次 234 · AppState 回流与 Host API 通配回归清除 · 2026-07-25

本轮没有机械改写 261 个合法 Host 入口；直接删除共享工作树中重新出现的 service-locator/公共 API 回流：

- `AppState.orchestrator` 与 `AppState.runtime_profile` 从 `pub(crate)` 收紧为真正私有；18 个生产文件中的直接 `runtime_profile` 字段读取全部改走既有 `runtime_profile()` 窄用途方法，直接字段读取为 0。
- `a4n-runtime` 根模块删除重新出现的 `pub use crate::actions::*`，恢复已有 `host_api.rs` 显式白名单；同时删除无消费者的完整 `OrchestratorContext` 根重导出。
- 四个已部署历史 migration 文件保持注册，末尾顺序固定为 `video_workflow_exclusive_claim` → `video_workflow_task_binding` → `neutral_video_segment_plan` → `shooting_script_transition_type_canonicalization`，解决“数据库已应用但二进制缺文件”的根因。
- `AppState` 观测值仍为 261 个 actions 文件 / 868 次命中；实际待迁主队列保持 0 rules / 0 透传壳。剩余引用是 Host 组装、权限、队列、provider、文件 IO、事件和 DTO 边界，不以改名或新增 facade 伪造下降。

验证结果：`cargo check -p a4n-runtime --no-default-features` 通过；`cargo check -p a4n-server --tests -p a4n-tauri --no-default-features -p a4n-migration` 通过（1m33s）；migration 历史注册测试、AppState 私有字段结构测试和 Host API 禁止通配导出测试均 1/1 通过；`git diff --check` 与 `lib.rs`/`state/app.rs` 定向 `rustfmt --check` 通过。静态搜索确认 Host API 通配导出、完整 `OrchestratorContext` 根重导出、AppState `orchestrator/runtime_profile` crate-visible 字段和生产代码直接字段读取均为 0。Runtime 测试编译仍报告一个既有 `sample_part` dead-code warning；当前无 PostgreSQL 环境，工作包 09 与总体 16/44 状态不变。

## 批次 235 · IG-CORE-01 规范事件与 Host DTO 合同前置 · 2026-07-25

本轮只落地可独立编译的合同前置，不接生产入口、不做旧/新兼容投影，也不删除旧事件链：

- `a4n-service-protocol::events` 增加 `GenerationTaskEvent`、`GenerationEvent`、`GenerationTaskSnapshot` 及 V1 payload、稳定错误码注册表和纯合同校验；校验覆盖 UUID v4、sequence、safe integer、UTF-8 offset/SHA-256、usage、progress、artifact、batch、terminal/snapshot 一致性和大小上限。
- `a4n-api-contract::generation_tasks` 增加固定 Host DTO；DTO 直接引用 service protocol 类型，不复制事件定义。
- 新增唯一 `export_generation_schema` 入口，生成 draft 2020-12 组合 JSON Schema；生成产物可被 Node 解析，所有对象 schema 保持 `additionalProperties=false`。
- 旧 `ServiceTaskEvent` 仍是迁移期合同；没有新增 `ServiceTaskEvent -> GenerationTaskEvent`、SSE/Tauri 投影、session、publisher、Web SSE、Tauri Channel 或前端 reducer。

验证结果：`cargo check -p a4n-service-protocol -p a4n-api-contract --all-targets` 通过；`cargo test -p a4n-service-protocol --lib` 67/67 通过；两个生成器 `--check`、`scripts/checks/check-api-contract-boundary.sh`、触及文件 `rustfmt --check` 和 `git diff --check` 通过。

仍未关闭：IG-CORE-01 的 session/replay/snapshot 持久化、统一 publisher、Web/Tauri 新传输、前端唯一 client/reducer，以及 IG-CUTOVER 的旧合同删除和 PostgreSQL/process 跨进程证据。工作包 09 仍为进行中，总体 V2 仍为 16/44。

## 批次 236 · 规范合同 TypeScript、Ajv 与共享 fixture 前置 · 2026-07-25

本轮继续只补合同边界，不接生产 transport、页面或旧事件投影：

- 唯一 Rust schema exporter 直接驱动 `json-schema-to-typescript`，生成 `generationContracts.ts`；`--check` 在临时目录重建 schema/TypeScript 并逐字节检查漂移，没有手写第三套 DTO。
- 前端增加一个 Ajv 2020 严格解析入口，复用生成 schema 和类型，显式限制事件 64 KiB、snapshot 4 MiB；当前没有接入 client、store、hook 或页面。
- Rust 与 Node 共用三份 V1 event fixture 和六组 UTF-8 向量，覆盖中文、emoji、combining character、跨 chunk byte offset、未知字段和 JavaScript unsafe integer。
- generation 合同检查已并入现有 `check:web-api-parity` 和 API contract boundary 门禁，没有新增 CI 工作流或第二套生成器。

验证结果：generation drift/Ajv/fixture 门禁通过；`cargo test -p a4n-service-protocol --lib` 68/68 通过；`frontend npm run build` 通过；API contract boundary、脚本语法、定向 `rustfmt --check` 和 `git diff --check` 通过。

仍未关闭：阶段 1 的 session admission、sequence lease、checkpoint/replay/snapshot 持久化、publisher、Web/Tauri 新传输、前端 client/reducer/store，以及领域原子迁移和 `IG-CUTOVER`。本批没有 PostgreSQL/process 证据，工作包 09 仍为进行中，总体 V2 仍为 16/44。

## 批次 237 · GenerationTaskSession、replay 与 publisher 内存 owner · 2026-07-25

本轮在 `a4n-orchestrator::generation::events` 建立唯一内存 owner，不接旧事件或领域生产入口：

- `GenerationTaskSession` 从已持久化的 started/snapshot 初始化，单锁分配 sequence、UTF-8 byte offset/chunk index，累计文本 SHA-256、usage、progress、artifact、checkpoint 和唯一终态 snapshot。
- replay 固定为 2,048 events/4 MiB，持久 covers 之前不得淘汰；`watch<u64>` 只通知 latest sequence，正文始终从同一 replay 读取，cursor 落后或超前时返回 snapshot。
- admission/内存账本固定 64 active sessions、64 KiB 基础预留、512 MiB 全局硬上限；prepared snapshot/replay 峰值也计入账本，terminal 立即释放 active slot。订阅门禁固定 128/task、1,024/Host。
- 普通 `GenerationEventPublisher` 只暴露 progress/text/usage；sequence lease 以 256 为 block、保留 16 个 emergency slots，单一 replenisher 串行扩租。`CommittedEventPermit` 在 DB 事务前冻结候选 sequence/snapshot，rollback 不消费，commit 后一次替换 session；没有新增兼容投影、broadcast 或第二 replay。

验证结果：`cargo check -p a4n-orchestrator --lib` 通过；定向 session 测试 8/8 通过，覆盖 Unicode offset/hash、usage、permit 回滚与等待、prepared memory、lease 扩租、replay floor/snapshot、watch 注册竞态、65th admission、129/task 与 1025/Host subscription、terminal slot 释放；定向 `rustfmt --check` 和 diff whitespace 检查通过。首次测试 60 秒只完成编译，缓存后进入 runner并取得上述结果。

仍未关闭：sequence lease 的 PostgreSQL 实现、checkpoint/start-request 表与事务、session registry/TTL、artifact/checkpoint/terminal coordinator、Web/Tauri transport、前端 client/reducer/store、各领域 `start_*` 和 `IG-CUTOVER`。本批没有生产接线或 PostgreSQL/process 证据，工作包 09 仍为进行中，总体 V2 仍为 16/44。

## 批次 238 · generation checkpoint/start-request PostgreSQL owner · 2026-07-25

本轮把增量生成的持久化不变量收敛到既有 `a4n-db::generation`，不接领域生产入口：

- 新增 `generation_task_checkpoint` 与 `generation_start_request` SeaORM entity；单个 forward-only `generation_task_persistence` migration 同时建立两张表、user/project 独立 lifecycle gate、状态/sequence/target/result/hash/谱系 CHECK、task/self/retry 外键、active business partial unique index和 owner/project/status/expiry 查询索引。旧 migration 顺序保持不动，新 migration 只追加在末尾。
- 新增唯一 `a4n-db::generation::tasks` persistence owner：business key 与 input fingerprint 使用 RFC 8785 canonical JSON；后端生成 UUID v4 task id；按 user→root project→project→A4SR→A4AT 固定锁序执行 lifecycle gate 检查、start-request 幂等复核、16 alias 上限、active business 冲突、attempt 分配、retry/task-tree 校验，并在同一事务写 `task_queue(pending)`、checkpoint `reserved` 和 start mapping。
- reservation 只从同一份持久 snapshot 初始化 sequence 1：`checkpoint_revision=1`、`covers=1`、`reserved=257`；public/execution JSON 读取前先按同一个 RFC 8785 helper 验 SHA-256，再反序列化并复核 snapshot 与规范列一致。
- sequence lease 使用单条 PostgreSQL CAS：同时匹配 task/revision/high-water/active status，成功后只做 `reserved += 256`、`revision += 1` 并 `RETURNING` 新值；没有第二个 sequence 分配器、逐事件表、repository trait、兼容 facade 或新 cleanup 机制。

验证结果：`cargo check -p a4n-db -p a4n-migration --lib` 通过；`cargo test -p a4n-db generation:: --lib -- --nocapture` 22/22 通过；migration SQL 不变量与历史注册顺序测试各 1/1 通过；触及文件定向 `rustfmt` 与 `git diff --check` 通过。当前未配置 `A4N_TEST_DATABASE_URL`/`DATABASE_URL`，所以没有声称真实 PostgreSQL DDL、advisory-lock 并发或事务故障注入通过。

仍未关闭：session registry/terminal TTL、checkpoint writer、artifact/checkpoint/terminal coordinator、lifecycle gate 的 deleting/resetting/disabling 写侧与 destructive drain、Web/Tauri transport、前端 client/reducer/store、领域 `start_*`、真实 PostgreSQL/process 证据和 `IG-CUTOVER`。工作包 09 仍为进行中，总体 V2 仍为 16/44。

## 批次 239 · GenerationSessionRegistry/TTL 与 checkpoint writer · 2026-07-25

本轮继续复用批次 237/238 的唯一 session、replay 和 PostgreSQL checkpoint owner，不新增 repository trait、缓存或第二套 sequence/checkpoint 状态：

- `GenerationSessionRegistry` 直接持有唯一 `GenerationAdmissionController` 和标准库 `Mutex<BTreeMap<task_id, Arc<GenerationTaskSession>>>`；新 admission 前同步清理 30 分钟到期 terminal session，内存压力时只按 `(terminal updated_at, task_id)` 淘汰最旧且 public snapshot 已持久化的 terminal session，永不驱逐 active session。task id/attempt 不一致显式失败。
- `GenerationTaskSession` 原地增加 checkpoint revision、最后成功落盘时间和未持久 event bytes；普通事件继续写同一 replay，checkpoint 成功后按 `sequence > persisted covers` 重算剩余脏字节。2 秒/64 KiB、TTL 30 分钟及允许范围进入同一配置校验，没有第二个 dirty queue。
- sequence lease 和 checkpoint writer 共用每 session 唯一异步 DB mutation lock；lease replenisher 现在同时返回新 revision/high-water，避免数据库 revision 增长后内存游标仍停留在旧值。`CommittedEventPermit::commit` 一次接收 revision/covers/high-water/persisted_at，后续 lifecycle/artifact/terminal owner 不再各自漏同步 cursor。
- 新增直接调用 `a4n-db::generation` 的 `GenerationCheckpointWriter`：普通 snapshot 使用单条 active-row CAS；省略的 execution/provider metadata 由 PostgreSQL `COALESCE` 原地保留；CAS 冲突只 reload/前进，不盲写旧 JSON。数据库错误固定按 250ms/1s/3s 重试，耗尽后封闭普通 publisher并返回 `generation_checkpoint_unavailable`。
- `CheckpointEvent` 只在 checkpoint application 内构造；先取得 durable permit、提交同一份 event-applied public snapshot，再 append。同事务已成功但内存 commit 失败时只从已验 hash 的持久 snapshot 整体重建，不补造 sequence。migration 同时补齐 execution checkpoint 必须有 provider、provider job 必须有 provider/idempotency hash 的 PostgreSQL CHECK。

验证结果：`cargo check -p a4n-db -p a4n-migration -p a4n-orchestrator --lib` 通过；session/registry/checkpoint bookkeeping 定向测试 10/10、checkpoint writer 固定重试/状态合同测试 1/1、`a4n-db generation::` 23/23、generation persistence migration 与历史注册测试各 1/1 通过。一次过宽的 `generation::` 过滤额外选中 23 个既有 PostgreSQL 业务测试，它们仅因缺少 `A4N_TEST_DATABASE_URL` 失败；本批随后用完整模块路径复验通过。当前仍无 `A4N_TEST_DATABASE_URL`/`DATABASE_URL`，不能声称真实 DDL、CAS 并发、事务故障重试或 process runner 已通过。

仍未关闭：lifecycle gate 写侧与 destructive drain、artifact/lifecycle/terminal coordinator、cleanup outbox generation kind 完整接线、registry/runtime/queue submit、Web/Tauri transport、前端 client/reducer/store、领域 `start_*`、真实 PostgreSQL/process 证据和 `IG-CUTOVER`。工作包 09 仍为进行中，总体 V2 仍为 16/44。

## 批次 240 · PostgreSQL persistence 真实证据与 terminal snapshot 正文 TTL

本轮继续复用唯一 `a4n-db::generation` persistence owner 和既有 cleanup worker，没有新增第二个 worker、task tree 表、兼容投影或内存 fallback：

- 新增 `generation_snapshot_retention` forward-only migration，允许 terminal checkpoint 在 `expires_at` 到期后清空 `public_snapshot_json`，同时强制保留并校验 64 位 `public_snapshot_sha256`；active 行仍必须有完整 snapshot，过期正文读取返回明确的“无 snapshot”状态而非伪造恢复。
- `expire_generation_snapshot_bodies` 使用 PostgreSQL `FOR UPDATE SKIP LOCKED` 批量更新，只递增 `checkpoint_revision`，不改 `updated_at`、result 引用、start-request mapping 或 `task_queue`；runtime 复用既有 cleanup worker 的启动/30 秒轮询，不新增 TTL 线程。
- terminal snapshot 的 `completed` 保留 24 小时，`failed/cancelled/interrupted` 保留 7 天；terminal 事务原有的 execution checkpoint 清空、provider/artifact cleanup intent 登记和唯一 sequence 语义不变。
- 新增真实 PostgreSQL 测试覆盖 reservation 幂等、第二 start-request alias、sequence lease CAS、checkpoint/provider metadata、stale writer 拒绝、project lifecycle gate 阻止新 reservation、terminal snapshot 到期清正文但保留 hash/映射/task_queue；迁移已在本地 PostgreSQL 18.4 实例实际执行。

验证：

```text
timeout 120 cargo check -p a4n-db -p a4n-migration -p a4n-orchestrator -p a4n-runtime --no-default-features：通过
timeout 120 cargo run -p a4n-migration -- "$DATABASE_URL"：通过，数据库迁移完成
timeout 180 cargo test -p a4n-db --test generation_persistence -- --nocapture --test-threads=1：1/1 通过（真实 PostgreSQL 18.4）
rustfmt（本批触及 Rust 文件）：通过
```

明确未关闭：cleanup outbox 审计记录自身 TTL；provider capability cleanup adapter、staging artifact delete/artifact unbind canonical Host owner、reconcile polling、Web fetch-SSE、Tauri Channel、前端 client/reducer/store、各领域 `start_*` 和 `IG-CUTOVER` 仍待完成。CI 定义为 PostgreSQL 16，本轮本地 18.4 证据不能替代 CI/跨平台/process 证据。3.15、3.16、阶段 G 与总体 V2 继续保持 `16/44`、未完成。

## 批次 241 · generation task tree 保留期删除与审计门槛 · 2026-07-25

本轮继续复用 `a4n-db::generation` persistence owner 与既有 cleanup worker，没有新增 task tree 表、mapping 表、删除 worker 或内存状态机：

- `purge_expired_generation_task_trees` 只领取 terminal root，要求 root 与全部 descendants 都有 `expires_at <= NOW()`；用户处于 `disabling/disabled` 时跳过，项目处于 `deleting/resetting` 时跳过。
- 同一事务先锁整棵树并校验递归谱系完整，随后按 `generation_start_request` mapping → `task_dependency` → 深度降序、同深度 task id 升序删除 `task_queue`；checkpoint 依赖既有外键级联，不留下 child/mapping 孤儿。
- 任务关联 cleanup outbox 仍有未完成记录或完成时间未满 30 天时不删除；这保留 provider/artifact 处理审计，outbox 审计记录自身的 TTL 清理仍是后续缺口。
- Runtime 只在现有 cleanup worker 启动恢复和 30 秒轮询中调用该函数，未增加第二个定时线程。

验证：

```text
timeout 120 cargo check -p a4n-db -p a4n-orchestrator -p a4n-runtime --no-default-features：通过
timeout 120 cargo test -p a4n-db --test generation_persistence generation_task_tree_ttl_waits_for_descendants_and_audit_before_deleting -- --nocapture --test-threads=1：1/1 通过（真实 PostgreSQL 18.4）
```

仍未关闭：cleanup outbox 审计记录自身 TTL、provider capability registry/真实 cancel-delete-status-reconcile、staging artifact/artifact unbind canonical Host owner、DB/文件双故障与 worker/process 组合、Web/Tauri transport、前端 client/reducer/store、领域 `start_*`、`IG-CUTOVER` 和跨进程事件顺序证据。3.15、3.16、阶段 G 与总体 V2 继续保持 `16/44`、未完成。

## 批次 242 · cleanup outbox completed 审计 30 天 TTL · 2026-07-25

本轮继续复用唯一 `a4n-db::cleanup_outbox` owner 和既有 cleanup worker，没有新增表、migration、索引、线程或第二套保留期配置：

- `purge_completed_cleanup_audits` 用单条 PostgreSQL CTE 按 `completed_at,id` 领取 `status=completed` 且完成时间已满 30 天的记录，使用 `FOR UPDATE SKIP LOCKED`、有界 `LIMIT` 和 `DELETE ... RETURNING` 完成并发安全批量删除；pending/running/failed 与未满 30 天的 completed 记录不会被清理。
- task tree 审计门槛与 outbox 自身清理共用 `CLEANUP_AUDIT_TTL`，30 天窗口不再有两个配置来源。
- Runtime 仍只在现有 cleanup worker 的启动恢复和 30 秒轮询中调用，batch limit 为 100；task tree 删除、snapshot 正文过期与 outbox 审计清理没有新增状态机或独立定时任务。
- 新增真实 PostgreSQL 集成测试，证明 limit=1 时只删除最旧过期 completed 记录，下一批删除剩余过期记录，29 天 completed 审计继续保留。

验证：

```text
timeout 120 cargo test -p a4n-db completed_cleanup_audit_ttl_prunes_only_expired_rows_in_batches -- --nocapture --test-threads=1：1/1 通过（真实 PostgreSQL 18.4）
timeout 120 cargo check -p a4n-db -p a4n-orchestrator -p a4n-runtime --no-default-features：通过；只有既有 generation 生产接线 dead-code 警告
本批触及 Rust 文件 rustfmt：通过；未运行 workspace 全量格式检查
```

仍未关闭：provider capability registry/真实 cancel-delete-status-reconcile、staging artifact/artifact unbind canonical Host owner、所有持久创建窗口的 durable intent/created-media 证明、DB/文件双故障与 worker/process 组合、Web/Tauri transport、前端 client/reducer/store、领域 `start_*`、`IG-CUTOVER` 和跨进程事件顺序证据。本地 PostgreSQL 18.4 不替代 CI PostgreSQL 16 或跨平台/process 证据；3.15、3.16、阶段 G 与总体 V2 继续保持 `16/44`、未完成。

## 批次 243 · generation cleanup 有界错误重试与脱敏 · 2026-07-25

本轮继续复用同一 cleanup outbox claim/lease/retry owner，没有新增 provider API、worker、队列或第二状态机：

- 新增 `retry_or_fail_claimed_generation_cleanup`，generation cleanup 的失败重试统一在 DB owner 内按 `1m/5m/30m/2h/12h` 五个重试窗口推进 `attempt_count`；第五次重试再次失败后显式写 `status=failed`，不再自动循环。
- Runtime 的 provider/staging/artifact 缺少 canonical owner 时仍不伪造成功；它调用上述专用迁移，记录重试或耗尽告警。provider capability unsupported 仍沿一次 claim 的 `completed/unsupported` 语义，不替换为 delete/reconcile。
- DB owner 对 `last_error` 使用既有 service-protocol 脱敏 helper，URL、API key 等敏感值不会进入 outbox 审计文本；项目/媒体 cleanup 的既有重试路径不改变。
- 新增真实 PostgreSQL 测试，逐个覆盖五个退避窗口、重试耗尽、lease 清理和错误脱敏。

验证：

```text
timeout 120 cargo test -p a4n-db generation_cleanup_retry_uses_bounded_schedule_and_redacts_error -- --nocapture --test-threads=1：1/1 通过（真实 PostgreSQL 18.4）
timeout 180 cargo check -p a4n-db -p a4n-orchestrator -p a4n-runtime --no-default-features：通过
本批触及 Rust 文件 rustfmt：通过；首轮 120 秒检查遇到 Cargo 构建锁超时，清理后重跑取得通过；未运行 workspace 全量格式检查
```

仍未关闭：provider capability registry/真实 cancel-delete-status-reconcile、staging artifact/artifact unbind canonical Host owner、所有持久创建窗口的 durable intent/created-media 证明、DB/文件双故障与 worker/process 组合、Web/Tauri transport、前端 client/reducer/store、领域 `start_*`、`IG-CUTOVER` 和跨进程事件顺序证据。工作包 09、3.15、3.16、阶段 G 与总体 V2 继续保持 `未完成` / `16/44`；本地 PostgreSQL 18.4 不替代 CI PostgreSQL 16、Windows/macOS 或真实 process 证据。

## 批次 244 · 删除无消费者 generation/cleanup facade，恢复 Host 编译 · 2026-07-25

状态：`未完成`

本批按 Ponytail full 先删没有生产消费者的代码，再修正直接依赖；没有为了降低 `AppState` 数字机械搬迁合法 Host 边界：

1. 删除 `GenerationApplication` 中无调用方的 sequence replenisher、execution runner、spawn/settle/fail facade，以及 `PersistentTaskQueue::acquire_generation_task_with_cancel`。保留真实使用中的 generation DB owner、terminal coordinator、session/lifecycle/cancel 与队列终态投影。
2. 删除 generation cleanup 的无消费者 `OrchestratorContext::fail_claimed_cleanup` 和 `ProjectFamilyRuntimeFacade::fail_claimed_cleanup` 透传层；底层 `a4n-db::cleanup_outbox::fail_claimed_cleanup` 仍保留给 claim 解析失败路径。
3. 为 Server/Tauri 补齐直接使用的 `a4n-api-contract` workspace 依赖，避免通过传递依赖编译；未新增兼容 facade 或 fallback。
4. `AppState` 观测值更新为 263 个 actions 文件 / 888 次命中；本地纯 rules 与 Orchestrator 透传壳仍为 0 / 0。

验证：

```text
timeout 180 cargo check -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri --no-default-features：通过
cargo test -p a4n-queue persisted_terminal_projection_removes_pending_task --lib -- --nocapture：1/1 通过
cargo test -p a4n-migration migrations_keep_user_default_ai_config_history_marker --lib -- --nocapture：1/1 通过
orchestrator/runtime 定向测试：120 秒冷编译窗口内未进入 runner，不记为通过
scripts/checks/refactor-metrics.sh：主队列 0 / 0；AppState 263 / 888
rustfmt --edition 2024 --check（本批触及文件）：通过；git diff --check：通过
```

仍未关闭：真实章节 `start_*` consumer、统一 generation transport/replay、Tauri Channel、前端 client/reducer/store、artifact/provider canonical owner、DB/文件双故障与 worker/process 证据、`IG-CUTOVER`。工作包 09、3.15、3.16、阶段 G 与总体 V2 继续为 `未完成` / `16/44`。

## 批次 245 · cleanup PostgreSQL/Server 强杀重启与双故障实跑 · 2026-07-25

状态：`未完成`

本批不新增测试框架或生产代码，直接复用批次 231–233 已有的 `cleanup_recovery` runner 和本地 PostgreSQL：

1. 真实启动两个 `a4n-server` 进程，证明同一 data directory 的第二个 Host 被拒绝，第一个进程强杀后新 Host 可重新取得操作系统文件锁。
2. 已存在 cleanup intent 的文件在第一次启动完成；物理删除失败写回 pending/attempt/error，修复目标并使 retry 到期后，第二个 Server 以 attempt 2 完成同一记录。
3. 真实 `/api/commands/upload_image` 在文件已写后，通过 PostgreSQL trigger 同时注入 generated-image 记录失败和 cleanup-intent 写入失败；第一进程退出后，第二进程启动 scanner 将 orphan 写入唯一 cleanup outbox 并删除文件。
4. 受管媒体 orphan、非媒体 orphan project 目录、重复启动恢复和 cleanup 审计状态都由同一组 process 断言覆盖；没有固定 sleep、测试专用 API、第二 worker 或内存补偿队列。

验证：

```text
timeout 300 cargo test -p a4n-server --test cleanup_recovery -- --nocapture --test-threads=1：3/3 通过（真实 PostgreSQL 18.4；测试进程 4.27s，总编译约 2m10s）
```

仍未关闭：所有持久创建入口的 durable intent/created-media 逐项证明、未被本 runner 覆盖的 DB 成功/文件失败组合、provider/artifact canonical Host owner、统一 generation transport/replay、Tauri Channel、前端 client/reducer/store、领域 `start_*`、`IG-CUTOVER` 和生成事件跨进程顺序证据。本地 PostgreSQL 18.4 不替代 CI PostgreSQL 16、Windows/macOS 或远端 provider/artifact 证据；工作包 09 与总体 V2 仍为 `未完成` / `16/44`。

## 批次 247 · Web generation read/SSE transport · 2026-07-25

状态：`未完成`

本批只推进全量增量生成计划阶段 4 的 Web 侧最小闭环，复用既有 `GenerationTaskSession`、replay、`watch<latest_sequence>`、checkpoint 和取消 coordinator，没有引入 broadcast、第二事件投影或兼容 fallback：

1. `OrchestratorContext`/Runtime 新增统一 generation task 授权读取：先校验 checkpoint owner，再校验显式 `attempt`，最后按项目 Viewer 权限校验；task id 不作为权限凭证。
2. 新增固定 Web 路由：`GET /api/generation-tasks/{task_id}`、`/snapshot`、`/events` 和 `POST /cancel`。summary 从规范 checkpoint 列投影；snapshot 复用已有 hash/TTL 语义；cancel body 的 task id 必须与 path 相等。
3. SSE 只接受 `Last-Event-ID: {attempt}:{sequence}`，事件固定为 `generation_task_event`、`id={attempt}:{sequence}` 和规范 `GenerationTaskEvent` JSON；初始 replay 落后时显式返回 `generation_snapshot_required`，后续 replay floor 越过则关闭连接等待 snapshot 恢复。
4. generation 路由使用既有 `HostErrorResponse` 和注册错误码，区分 400/401/403/404/409/410/413/429/503；保留旧 `/api/events/{channel}` 给尚未迁移的非 generation 功能。

验证：

```text
timeout 240 cargo check -p a4n-server：通过
timeout 120 cargo test -p a4n-server generation::tests -- --nocapture：2/2 通过
git diff --check：通过（已跟踪改动）；cargo fmt --all -- --check 仍被共享工作树既有未格式化文件阻塞，未把它记为通过
```

仍未关闭：`active/history` 分页、无 live session 的 snapshot-only 重建、Tauri Channel、前端唯一 client/reducer/store、真实领域 `start_*` consumer、provider/artifact canonical Host owner、所有持久创建窗口证明、IG-CUTOVER 旧合同删除，以及跨进程顺序/usage/checkpoint/artifact/cancel-ack 证据。工作包 09、3.15、3.16、阶段 G 与总体 V2 继续为 `未完成` / `16/44`。

## 批次 246 · Runtime 内部复用现有窄 facade · 2026-07-25

状态：`未完成`

本批按 Ponytail full 只收窄能证明依赖集合的内部函数，不新增 facade、application、trait 或第二套权限逻辑：

1. RAG search 的准备、embedding identity 和执行链改为接收已有 `RagRuntimeFacade`；公开 Host action 仍接收 `AppState` 并负责权限校验。
2. Canvas preview 的文档读取 helper 改为接收已有 `CanvasRuntimeFacade`；权限边界继续只在公开 preview action。
3. Object/Scene profile 的项目冲突与下游图片查询 helper 分别复用已有 `AssetRuntimeFacade`、`SceneProfileRuntimeFacade` 和 `ProfileMaterialRuntimeFacade`；文件删除、冲突确认与下游清理 owner 不移动。
4. 最新度量为 Runtime actions 532 个生产文件 / 27,876 行、含测试 33,978 行；`AppState` 从 263 文件 / 888 命中降为 258 / 873，主队列纯 rules 与 Orchestrator 透传壳仍为 0 / 0。

集中验证：

```text
timeout 180 cargo check -p a4n-runtime --no-default-features：通过
timeout 120 cargo test -p a4n-runtime --no-default-features --lib -- --nocapture --skip 'actions::project_duplication::tests::'：465/465 通过；8 个显式 PostgreSQL 用例因未配置 A4N_TEST_DATABASE_URL 跳过
scripts/checks/check-task-kind-boundary.sh：通过
git diff --check：通过
scripts/checks/refactor-metrics.sh：Runtime actions 532 / 27,876，含测试 33,978；AppState 258 / 873；主队列 0 / 0
```

完整 Runtime lib 首轮实跑为 462 通过 / 11 失败：8 个失败来自缺少显式 PostgreSQL 测试库；另外 3 个是共享工作树已有代码与旧测试断言不一致，本批已把远端无密钥 egress scope 和两个导入 Host 的 `write_new_data_file` 断言更新到现行合同，随后非 PostgreSQL 全集 465/465 通过。`check-refactor-docs.sh` 仅因计划/进度/工作包文档尚未进入 Git 索引而失败；按工作区保护规则未擅自 stage。

工作包 09、3.15、3.16、阶段 G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 248 · 启动媒体恢复证据去假通过 · 2026-07-25

状态：`未完成`

本批按 Ponytail full 不改生产 owner，只收紧已有生命周期测试：

1. `storage_paths` 单测覆盖 `images/videos/portraits/audio/media/assets` 六个受管根目录，并同时证明非媒体目录不进入扫描、完整 assembly cache 不被普通 orphan scanner 重复处理、不完整 cache 仍进入恢复候选。
2. Server 强杀/重启 runner 增加六类媒体 orphan、过期 assembly output/manifest 和过期 `video_upload_*`；继续复用现有启动恢复顺序、cleanup outbox 和标准库 `FileTimes`，没有新增测试框架或第二 cleanup owner。
3. 六类媒体与 assembly cache 改为属于数据库中的存活项目，避免末尾的孤儿项目目录清理掩盖媒体 scanner 失效；非媒体 orphan project 仍由独立项目目录覆盖。

验证：

```text
timeout 60 cargo test -p a4n-runtime --no-default-features storage_paths --lib -- --nocapture：4/4 通过
timeout 60 cargo test -p a4n-runtime --no-default-features expired_ --lib -- --nocapture：1/1 通过
timeout 60 cargo test -p a4n-runtime --no-default-features uploaded_video_recovery_only --lib -- --nocapture：1/1 通过
timeout 60 cargo test -p a4n-server --test cleanup_recovery --no-run：通过
scripts/checks/check-task-kind-boundary.sh：通过
git diff --check：通过
scripts/checks/refactor-metrics.sh：Runtime actions 532 / 27,933，含测试 34,042；AppState 258 / 877；主队列 0 / 0
```

当前未配置 `A4N_TEST_DATABASE_URL`，所以扩展后的 Server process runner 未进入执行，不能把批次 245 的旧版 3/3 结果算作本批通过。所有持久创建入口、剩余 DB/文件故障组合、provider/artifact canonical owner、Tauri/前端 generation transport、真实领域 `start_*`、`IG-CUTOVER` 与跨进程事件顺序仍未关闭；工作包 09 和总体 V2 继续为 `未完成` / `16/44`。

## 批次 249 · Web generation active/history 与容错 summary · 2026-07-25

状态：`未完成`

本批按 Ponytail full 复用既有 checkpoint、项目协作权限、RFC8785 helper 和 Host DTO，完成 Web generation 读取面的 active/history，不建立第二列表缓存、分页 owner 或事件来源：

1. DB owner 在同一查询中先限制 `requested_by_user_id`，再按当前 root project owner/member Viewer 权限过滤；之后才应用 project filter、复合 cursor 和 limit。active statuses 固定为 `reserved/running/finalizing/cancelling`，最多 64 条；history 固定为 `completed/failed/cancelled/interrupted` 且 checkpoint 未过期，默认 50、范围 1～100。
2. 排序和翻页固定使用 `updated_at_ms DESC, task_id ASC` 与 `updated_at_ms < cursor.updated_at_ms OR (updated_at_ms = cursor.updated_at_ms AND task_id > cursor.task_id)`；Runtime 复用 `a4n-db::generation::canonical_json_bytes`、已有 base64/sha2 依赖实现 `v1.payload.checksum` cursor，没有新增依赖。
3. summary/detail/list 使用 tolerant checkpoint loader：先做 owner → attempt → project 授权，再验证 snapshot；public/execution snapshot 损坏只投影 `snapshot_state=corrupt`，TTL 到期投影 expired，规范 status/started/result_ref 仍可查询。events/snapshot 同样改为授权后才做严格 snapshot 校验。
4. Server 注册 `/api/generation-tasks/active` 与 `/history`，非法 cursor 返回 400 `generation_cursor_invalid`；SSE 响应补齐 `no-cache, no-transform`。

验证：

```text
timeout 180 cargo check -p a4n-server：通过
timeout 180 cargo test -p a4n-runtime history_cursor_round_trips_and_rejects_tampering -- --nocapture：1/1 通过
timeout 180 cargo test -p a4n-server generation::tests -- --nocapture：2/2 通过
timeout 120 cargo test -p a4n-db reservation_builds_started_snapshot_and_first_sequence_lease --lib -- --nocapture：1/1 通过
本批触及 Rust 文件定向 rustfmt：通过
```

当前没有 `A4N_TEST_DATABASE_URL`/`DATABASE_URL`，真实 PostgreSQL 同毫秒分页、跨用户/跨项目负向和 HTTP process 测试未运行。snapshot-only session、Tauri Channel、前端唯一 client/reducer/store、真实领域 `start_*`、provider/artifact canonical Host owner、全部持久创建窗口、`IG-CUTOVER` 与跨进程事件顺序仍未关闭；工作包 09、3.15、3.16、阶段 G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 250 · snapshot-only session 恢复与查询权限/上限单一来源收敛 · 2026-07-25

状态：`未完成`

本批只收敛工作包 09 已有 generation session/Host DTO 的恢复门禁，不新增事件源、状态机、worker、列表缓存或兼容投影：

1. `GenerationSessionRegistry` 增加 `insert_restored_or_get`；并发读取同一已授权 task 时只保留一个恢复 session。`GenerationTaskSession::from_snapshot_only` 只保存持久 snapshot，不伪造 replay 前缀；未完成 snapshot 在客户端以精确 `covers_through_sequence` 建立订阅前统一返回 `GenerationReplay::SnapshotRequired`。
2. 活跃 snapshot-only session 只有在 `after_sequence == Some(snapshot.covers_through_sequence)` 时解除门禁；终态 snapshot 即使 cursor 等于 covers 也永不重新开放 events，客户端必须读取 snapshot 后停止订阅。`mark_snapshot_persisted`、`CommittedEventPermit::commit` 和持久 snapshot 重建同步维护该 recovery covers。
3. 已授权 task 在没有 live session 时由同一份 `PersistedGenerationTask` 恢复唯一 session；Runtime、Server 和 Tauri subscription 入口不再按 task id 盲读第二份任务或绕过授权路径。Tauri Channel 命令/registry 已接入该调用，前端 generation client/reducer/store 代码也已在工作区，但完整跨进程、断线重订阅和页面使用证据仍未关闭。
4. collaboration 访问条件抽成共享 `accessible_project_condition`/`accessible_project_ids`，generation active/history 先按请求用户和可访问项目过滤，再应用 project/cursor/limit；active 上限 64 统一引用 service-protocol 的 `DEFAULT_GENERATION_ACTIVE_SESSION_MAX`，删除 DB 与 session config 的第二个字面量来源。

验证：

```text
timeout 180 npm run build（frontend）：通过，tsc + Vite build（2178 modules）
timeout 120 npm run test:run -- generationEventReducer sseParser（frontend）：2 files / 7 tests 通过
timeout 180 cargo check -p a4n-server -p a4n-tauri --no-default-features：通过（缓存后 0.71s）
timeout 120 cargo test -p a4n-orchestrator snapshot_only_session --lib -- --nocapture：2/2 通过
timeout 180 cargo test -p a4n-tauri --no-default-features -- --nocapture：25/25 通过（含 Channel 窗口隔离/销毁清理与 service→Tauri 顺序）
rustfmt --edition 2024（本批触及 Rust 文件）：通过
```

当前没有 `A4N_TEST_DATABASE_URL`/`DATABASE_URL`，`ANY(bigint[])` active/history 查询的真实 PostgreSQL 绑定、同毫秒分页与跨用户/跨项目负向尚未实跑；Tauri Channel process/断线重订阅、前端唯一 client 全量页面接入、真实领域 `start_*`、provider/artifact canonical Host owner、全部持久创建窗口、`IG-CUTOVER` 和跨进程 usage/checkpoint/artifact/cancel-ack 证据仍未关闭。工作包 09、3.15、3.16、阶段 G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 251 · generation transport parity 与宿主门禁去假失败 · 2026-07-25

状态：`未完成`

本批不增加第二套 Web 命令映射、事件来源、replay buffer 或 cleanup owner：

1. generation 的 Tauri-only 查询/订阅调用复用既有 `runtimeInvoke`，Channel 由 runtime adapter 构造；唯一 generation client 不再直接依赖 Tauri 包。
2. parity 门禁区分 portable `apiInvoke` 与 Tauri-only `runtimeInvoke`，分别检查 Web/Tauri 双注册和桌面注册。Web 仍只有 generation client 的直接 REST/SSE 实现，没有复制到 `webInvoke`。
3. 修复 Video 命令测试只扫描顶层文件造成的假失败，补齐实际 `pub use` 的 generation/prompts 子模块；未改生产命令或 registry。
4. unsubscribe 的窗口销毁兜底保留，但失败会显式 warning，不再静默吞错。

验证：

```text
timeout 180 cargo check -p a4n-server -p a4n-tauri --no-default-features：通过
timeout 90 cargo test -p a4n-tauri --no-default-features -- --nocapture：25/25 通过
frontend npm run test:run：2 files / 7 tests 通过
frontend npm run build：tsc + Vite 通过（2178 modules）
frontend check:generation-contract / check:web-api-parity：通过；Web/Tauri/parity 缺口均为 0
rustfmt 定向检查、node --check、git diff --check：通过
```

联合 Rust 命令中的 Server 25 个单元测试通过，随后 3 个 cleanup process 用例因没有 `A4N_TEST_DATABASE_URL` 失败；当前也没有真实 Tauri 窗口或浏览器 process。真实领域 `start_*`、页面迁移、provider/artifact canonical Host owner、全部持久创建窗口、日志保留、`IG-CUTOVER` 与跨进程 usage/checkpoint/artifact/cancel-ack 仍未关闭；工作包 09、3.15、3.16、阶段 G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 252 · generation active/history PostgreSQL 授权与同毫秒分页实跑 · 2026-07-25

状态：`未完成`

本批只给批次 249/250 的既有 DB 查询补真实 PostgreSQL 证据，不新增 repository、分页框架、权限规则或测试数据库管理器：

1. 在现有 `generation_persistence` runner 增加 `generation_task_lists_filter_access_before_cursor_and_limit`，继续复用 `connect_test_database` 的隔离 schema 和完整 migration。
2. active 查询真实绑定 `ANY(bigint[])`，证明共享根项目 Viewer 可以看到自己请求的任务；同项目其他用户任务和无协作权限项目任务在 project filter/cursor/limit 前被排除，task/checkpoint 行本身不能充当项目访问凭证。
3. history 创建三个相同 `updated_at` 毫秒的 terminal task，第一页 limit=2、第二页使用 `(updated_at_ms, task_id)` cursor；断言 `has_more`、稳定 `task_id ASC`、无重复、无遗漏，并排除不可访问项目任务。
4. 测试同时保留 reservation/lease/checkpoint/lifecycle gate/terminal TTL 与 task tree/outbox 审计门槛原有用例，避免只跑新路径掩盖既有持久化回归。

验证：

```text
A4N_TEST_DATABASE_URL=<本地 PostgreSQL 18.4> timeout 300 cargo test -p a4n-db --test generation_persistence -- --nocapture --test-threads=1：3/3 通过
rustfmt --edition 2024 --check crates/a4n-db/tests/generation_persistence.rs：通过
```

本地 PostgreSQL 18.4 不能替代 CI PostgreSQL 16 或 HTTP/Tauri process。真实 Host 断线重订阅与 hash、前端页面全量迁移、真实领域 `start_*`、provider/artifact canonical Host owner、全部持久创建窗口、`IG-CUTOVER` 和跨进程 usage/checkpoint/artifact/cancel-ack 仍未关闭；工作包 09、3.15、3.16、阶段 G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 253 · 扩展 cleanup Server 强杀重启与双故障 process 复验 · 2026-07-25

状态：`未完成`

本批没有新增生产代码、测试框架或 cleanup worker，直接在本地 PostgreSQL 18.4 运行批次 248 扩展后的既有 `cleanup_recovery` process runner：

1. 同一 data directory 的第二个 Server 被 OS 文件锁拒绝，原进程强杀后新 Server 能重新取得 ownership。
2. 物理删除失败保留同一 cleanup outbox intent；修复目标并跨进程重启后，以后续 attempt 完成删除。
3. 上传文件已写后同时注入业务记录失败和 cleanup-intent 写入失败；重启 scanner 重新登记唯一 intent 并删除 orphan。
4. runner 当前夹具同时包含六类受管媒体、过期 assembly output/manifest、过期 `video_upload_*` 与非媒体 orphan project 目录，避免孤儿项目清理掩盖媒体 scanner 回归。

验证：

```text
A4N_TEST_DATABASE_URL=<本地 PostgreSQL 18.4> timeout 360 cargo test -p a4n-server --test cleanup_recovery -- --nocapture --test-threads=1：3/3 通过（runner 3.53s）
```

本地 PostgreSQL 18.4 不能替代 CI PostgreSQL 16、Windows/macOS 或远端 provider/artifact 证据。所有持久创建入口的 durable intent/created-media 逐项证明、generation Host 断线重订阅、前端页面迁移、真实领域 `start_*`、provider/artifact canonical Host owner、`IG-CUTOVER` 与跨进程 generation usage/checkpoint/artifact/cancel-ack 仍未关闭；工作包 09、3.15、3.16、阶段 G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 254 · generation Server process 恢复订阅与 sequence lease 高水位收口 · 2026-07-25

状态：`未完成`

本批只修复恢复路径的真实不变量并补 Server process 证据，不新增事件源、replay buffer、worker 或兼容投影：

1. `GenerationTerminalCoordinator::interrupt_persisted_task` 在 Host 崩溃后消耗未落盘的 sequence lease 时，从数据库 `reserved_through_sequence + 1` 写入恢复终态；snapshot 先推进到 lease 高水位，再应用 terminal event，避免把合法的恢复序列间隙误判为 replay identity 错误。普通 live session 仍严格要求连续 sequence。
2. `crates/a4n-server/tests/cleanup_recovery.rs` 新增真实 `a4n-server` process 用例：启动前 active task 被 Host 恢复为 `interrupted`（covers=258），terminal snapshot 即使使用精确 covers 仍返回 `generation_snapshot_required`；进程就绪后创建的 active task 首次订阅返回同一错误，使用 `Last-Event-ID: 1:1` 建立 200 SSE；无项目权限用户读取 snapshot 返回 403。
3. 测试复用现有隔离 PostgreSQL schema、`spawn_server`/ownership lock、唯一 generation authorization/session/replay owner；`a4n-server` 仅补充已有 workspace `a4n-queue` dev dependency 用于 reservation fixture，没有生产依赖或第二套测试服务。

验证：

```text
A4N_TEST_DATABASE_URL=<本地 PostgreSQL 18.4> timeout 180 cargo test -p a4n-server --test cleanup_recovery server_process_recovers_snapshot_only_generation_subscription -- --nocapture --test-threads=1：1/1 通过（runner 2.01s）
rustfmt --edition 2021 crates/a4n-orchestrator/src/generation/terminal.rs crates/a4n-server/tests/cleanup_recovery.rs：通过
```

第一次冷编译命令在 120 秒预算内未进入 runner，不能记为通过；缓存后重跑成功。`cargo fmt --all -- --check` 仍被共享工作树中与本批无关的既有格式差异阻塞，因此没有把全库格式检查写成通过。该证据只覆盖 Server HTTP process，不替代真实 Tauri process、断线 hash/重订阅、CI PostgreSQL 16、页面迁移、领域 `start_*`、全部持久创建窗口、provider/artifact canonical Host owner、`IG-CUTOVER` 和跨进程 usage/checkpoint/artifact/cancel-ack；工作包 09 与总体 V2 仍为 `未完成` / `16/44`。

## 批次 255 · snapshot-only reserved cancel 与递归取消查询收口 · 2026-07-25

状态：`未完成`

本批继续复用唯一 generation session、terminal coordinator、队列投影和现有 Server process runner，不新增取消状态机、第二事件源或测试专用 API：

1. 修复 `generation_cancel_targets` 递归 CTE 的真实 PostgreSQL 别名错误：递归项统一从已连接的 `parent.path` 扩展和去环，保留 task id 排序及整棵树权限/owner 校验。
2. `OrchestratorContext::request_generation_cancel` 处理 snapshot-only session 没有内存 queue owner 的边界：reserved task 由同一 terminal coordinator 直接提交 `CancelledEvent`；running/cancelling task 在 queue owner 丢失时提交 `InterruptedEvent(reason=session_owner_lost)`；正常 running queue 仍只触发既有 cancellation token，终态由执行 owner 收口。
3. 扩展 `server_process_recovers_snapshot_only_generation_subscription`：真实 HTTP `POST /cancel` 返回当前 `cancelled` handle；同一 SSE 连接按 `sequence=2 cancel_requested → sequence=3 cancelled` 收到唯一 ack/终态；递归 child task 没有 live session 时按 lease 高水位写入唯一 `interrupted(sequence=258)`；终态 snapshot 的精确 cursor 仍返回 `generation_snapshot_required`，既有恢复 terminal、active exact cursor 和 outsider 403 断言保留。

验证：

```text
A4N_TEST_DATABASE_URL=<本地 PostgreSQL 18.4> timeout 180 cargo test -p a4n-server --test cleanup_recovery server_process_recovers_snapshot_only_generation_subscription -- --nocapture --test-threads=1：1/1 通过（runner 2.06s）
A4N_TEST_DATABASE_URL=<本地 PostgreSQL 18.4> timeout 180 cargo test -p a4n-server --test cleanup_recovery -- --nocapture --test-threads=1：4/4 通过（runner 5.35s）
A4N_TEST_DATABASE_URL=<本地 PostgreSQL 18.4> timeout 180 cargo test -p a4n-db --test generation_persistence -- --nocapture --test-threads=1：3/3 通过（runner 2.81s）
rustfmt --edition 2021 --check crates/a4n-server/tests/cleanup_recovery.rs：通过；定向 Server process 编译通过
```

本批仍不关闭 3.15/3.16：没有把 reserved fixture 冒充真实领域 `start_*` consumer，running/cancelling owner-lost 分支没有宣称跨进程执行证据；真实 Tauri/Web process、断线 hash/重订阅、CI PostgreSQL 16、页面全量迁移、全部持久创建窗口、provider/artifact canonical Host owner、`IG-CUTOVER` 和 usage/checkpoint/artifact/cancel-ack 跨进程顺序仍未执行。共享工作树全库 `cargo fmt --all -- --check` 仍受其他会话既有格式差异影响，未格式化无关文件；工作包 09 与总体 V2 继续为 `未完成` / `16/44`。

## 批次 256 · generation store 行为门禁 · 2026-07-25

状态：`未完成`

本批只补阶段 5 的前端 store 行为证据，复用已有唯一 `generationClient`、Zustand store 和 reducer，不新增取消逻辑、第二 authoritative state 或测试框架：

1. 新增 `frontend/src/store/generationTasks.test.ts`，验证同一 task 的多引用 retain/release；最后一个引用释放只把 view 标记为 `detached`，不触发任务取消。
2. 验证普通 generation event 只在既有 50ms flush 窗口后写入可观察 Zustand view，而 terminal event 立即 flush。
3. 验证 `clear` 同时移除 Zustand view、引用计数和模块内 authoritative state；clear 后 dispatch 显式返回 `null`。

验证：

```text
timeout 120 npm run test:run -- src/store/generationTasks.test.ts：3/3 通过
timeout 120 scripts/checks/refactor-metrics.sh --label "批次 256 完成后" --append：通过；主队列仍为纯 rules 0/0、Orchestrator 透传壳 0/0
```

本批没有 hook、页面、真实 `start_*`、Tauri/Web process 或 10,000 chunk render 证据，因此不关闭阶段 5/6、3.15/3.16；后续不得重复实现 store 的引用计数和 flush 规则，应继续补 hook/页面迁移与真实领域入口。工作包 09 与总体 V2 保持 `未完成` / `16/44`。

## 批次 257 · generation 合同产物重新生成 · 2026-07-25

状态：`未完成`

本批修复一个真实生成合同漂移，不手写 TypeScript 类型，也不新增兼容字段：

1. Rust `a4n-api-contract::chapter::StartChapterGenerationInput` 已被组合 schema exporter 声明，但此前 `frontend/src/api/generated/generation-contract.schema.json` 与 `generationContracts.ts` 没有同步该 `$ref`。
2. 使用项目既有 `scripts/contracts/generate-generation-contracts.sh --write` 重新生成 schema 和 TypeScript 产物，生成结果包含 `StartChapterGenerationInput`，并保留 `additionalProperties=false`/deny-unknown-fields 合同。
3. 生成产物重新通过 `--check`；既有 reducer、SSE parser 和本批 store 测试合计 3 files / 10 tests 通过。

验证：

```text
timeout 120 bash scripts/contracts/generate-generation-contracts.sh --check：通过
timeout 120 npm run test:run -- src/utils/generationEventReducer.test.ts src/utils/sseParser.test.ts src/store/generationTasks.test.ts：3 files / 10 tests 通过
```

本批仍没有真实章节 `start_*` 执行 owner、页面接入、Tauri/Web process 或 IG-CUTOVER；不要把生成产物同步误记为领域迁移完成。工作包 09、阶段 5/6、3.15/3.16 与总体 V2 继续为 `未完成` / `16/44`。

## 批次 258 · 章节正文统一 generation 执行链路与正式结果校正 · 2026-07-25

状态：`未完成`

本批只处理章节正文这一真实领域入口，复用既有 reservation、`GenerationTaskSession`、replay/publisher、checkpoint writer、terminal coordinator、queue projection 和 Host DTO；没有新增 callback、第二队列、第二 replay、兼容 wrapper 或页面私有事件合同：

1. `run_chapter_generation_task` 现在从 `StartChapterGenerationInput` 直接执行 `prepare → prompt → execute_llm_generation → checkpoint → validate → terminal.complete_with`，完成事件的 `result_ref` 与 `final_text_hashes` 均来自事务内正式章节正文 SHA-256。
2. 章节旧的 workflow stream、progress/chunk callback、随机 task-id、旧 stream DTO 与无事务 checkpoint restore 已从编译路径删除；失败、取消、中断继续由同一 terminal coordinator 收口，旧章节服务保留的非事务完成/恢复残留也已删除。
3. Runtime/Tauri/Server 的章节 start 入口统一接受生成合同 `StartChapterGenerationInput`；Runtime 使用受管 execution owner 持有 worker，Tauri command 与 Web `/generation-tasks/chapter` 都返回同一 `GenerationTaskHandle`。
4. 写作桌只通过 `generationClient.attach` 订阅唯一 store/reducer。完成态先强制 reload 正式章节，再按 `result_ref.kind/id/revision` 查找并计算正文 SHA-256；不一致或缺失时进入 `consistency_error`、恢复编辑器快照且保留草稿，不伪装成功；一致后才清理草稿并显示成功。
5. `useGenerationTask` 接受可空 handle，最后一个订阅引用只 detach；一致性/协议错误不会被 detach 覆盖为普通 detached 状态。
6. 写作桌切换项目时同步清除本地 generation handle/ref；旧 start 响应、流文本、终态校验和章节 reload 都按当前 project/task identity 拒绝迟到结果。章节切换不再通过 effect cleanup 中断正在执行的终态校验，避免任务已 terminal 但页面永久不收尾。
7. 章节 service 的 PostgreSQL 测试夹具复用现有文本项目常量，补齐 `project_kind/source_mode/source_sync_status` 当前非空合同；没有给业务路径增加默认值或兼容 fallback。

验证：

```text
timeout 120 npm run test:run -- src/utils/generationEventReducer.test.ts src/utils/sseParser.test.ts src/store/generationTasks.test.ts：3 files / 10 tests 通过
timeout 180 npm run build（frontend）：通过，tsc + Vite（2359 modules）
npm run check:generation-contract / npm run check:web-api-parity：通过，generation fixture/schema/current、Web/Tauri/parity 缺口均为 0
timeout 120 cargo test -p a4n-runtime --no-default-features state::convergence_tests::chapter_generation --lib -- --nocapture：2/2 通过
A4N_TEST_DATABASE_URL=<本地 PostgreSQL 18.4> timeout 60 cargo test -p a4n-orchestrator --lib --no-default-features save_content_edit_rejects_empty_content_without_changing_chapter -- --nocapture --test-threads=1：1/1 通过
timeout 180 cargo check -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri --no-default-features --message-format short：通过
inventory-generation-paths：145/145，missing=0、stale=0；canonical TaskKind boundary、章节旧入口扫描、rustfmt 与目标 diff check：通过
```

本批仍未关闭：真实 Tauri/Web process、浏览器/代理断线 hash 与重订阅、页面全量重挂载过程证据、所有持久 `start_*` 入口、provider/artifact canonical Host adapter、IG-CUTOVER 旧合同整体删除、跨进程 usage/checkpoint/artifact/cancel-ack 事件顺序，以及 CI PostgreSQL 16/Windows/macOS 证据。V2 继续为 `16 / 44`，工作包 09、3.15/3.16 与阶段 G 继续 `未完成`。

## 批次 259 · Writing Desk generation 重挂载与真实 Web 失败终态进程证据 · 2026-07-25

状态：`未完成`

本批复用既有 active summary、唯一 generation client/store/reducer、Server SSE、terminal coordinator 和受管 execution owner，没有新增 transport、页面私有恢复状态机或测试专用 endpoint：

1. 写作桌按当前项目筛选 `target.kind=chapter` 且 target id 匹配的 active summary，重挂载后恢复同一个 handle 并重新 attach；旧 workflow 事件、随机 task id 和同步/流式双分支不再参与章节正文路径。
2. 章节 API 统一接受 `StartChapterGenerationInput`，保留桌面本地 embedding 的既有适配；`chapterGenerationNumber` 增加规范 target 解析测试。
3. `cleanup_recovery` 的真实 Server process runner 创建合法文本项目/蓝图/章纲和本地不可达 provider，验证 `POST /generation-tasks/chapter → SSE` 的连续序列、唯一 `failed` terminal、连接关闭、summary/snapshot 一致，以及相同 `start_request_id` 幂等返回原 attempt；失败不写正式章节。

验证：

```text
A4N_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/postgres timeout 300 cargo test -p a4n-server --test cleanup_recovery server_process_streams_chapter_generation_failure_terminal -- --nocapture --test-threads=1：1/1 通过
npx tsc --noEmit：通过
npm run build：通过（2359 modules）
npm run check:web-api-parity：通过
npm run test:run -- src/api/chapter.test.ts src/utils/generationEventReducer.test.ts src/utils/sseParser.test.ts src/store/generationTasks.test.ts：4 files / 11 tests 通过
git diff --check（本批前端目标文件）：通过
```

`rustfmt --edition 2021 --check crates/a4n-server/tests/cleanup_recovery.rs` 仍被该共享测试文件原有格式差异阻塞；未用全文件重格式化覆盖并行改动。真实成功 Web/Tauri process、浏览器/代理断线 hash 与重订阅、完整页面重挂载 process 证据、灵感及其他持久 `start_*` consumer、provider/artifact canonical Host adapter、所有持久创建窗口、`IG-CUTOVER` 与跨进程 usage/checkpoint/artifact/cancel-ack 顺序仍未关闭；V2 保持 `16 / 44`。

## 批次 260 · 灵感对话真实页面 consumer 统一迁移与失败终态进程证据 · 2026-07-25

状态：`未完成`

本批只把灵感对话页面接入既有 canonical generation 链路，并清理迁移遗留，不新增 transport、第二 reducer/store、同步/流式兼容桥或测试专用 endpoint：

1. 灵感对话统一使用 `start_inspiration_generation`、`GenerationTaskSession`、唯一 `generationClient`/Zustand store/reducer、`raw_json` text stream 和既有 terminal/checkpoint/queue owner；删除旧 `converse_inspiration`、`converse_inspiration_stream`、`workflow:<task_id>` bridge、随机 task id 与页面双分支。
2. Server/Tauri 均返回同一 `GenerationTaskHandle`；页面按 canonical `target.kind=inspiration_conversation` 与精确 `target.id=project_id` 恢复 active task，终态读取正式消息并校验 `result_ref.revision` SHA-256；缺失或不一致进入 `consistency_error`，失败/取消/中断显式展示。
3. DB 在同一事务锁定 conversation，按 user→assistant 顺序写入消息、更新 `message_count` 并返回 assistant message id；生成合同 schema/TypeScript 继续由既有脚本生成，未手写第二份类型。
4. 补齐 canonical target 负向测试，并删除 Runtime 测试对已移除 `ConverseInput` 的残留引用，改为验证当前 `PreviewInspirationRequestInput` 的 deny-unknown-fields 合同。
5. 复用 `cleanup_recovery` 既有 Server process runner，使用本地不可达 provider 验证真实 `POST /generation-tasks/inspiration → SSE → failed terminal` 连续序列、单一终态、summary/result_ref 一致且失败不写正式消息。

验证：

```text
npm run check:web-api-parity：通过
npx tsc --noEmit：通过
npm run test:run -- src/api/inspiration.test.ts src/api/chapter.test.ts src/utils/generationEventReducer.test.ts src/utils/sseParser.test.ts src/store/generationTasks.test.ts：5 files / 12 tests 通过
npm run build：通过（2359 modules）
timeout 60 cargo check -p a4n-api-contract -p a4n-db -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri --no-default-features：通过
timeout 60 cargo test -p a4n-runtime --no-default-features state::convergence_tests::inspiration_runtime_uses_narrow_facade --lib -- --nocapture：1/1 通过
timeout 60 cargo test -p a4n-db --lib inspiration -- --nocapture（A4N_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/postgres）：4/4 通过
timeout 60 cargo test -p a4n-orchestrator --lib inspiration -- --nocapture：9/9 通过
A4N_TEST_DATABASE_URL=postgres://postgres@127.0.0.1:55432/postgres timeout 180 cargo test -p a4n-server --test cleanup_recovery server_process_streams_inspiration_generation_failure_terminal -- --nocapture --test-threads=1：1/1 通过（runner 1.60s）
rustfmt --edition 2024 --check（本批触及 Rust 文件）、git diff --check：通过
```

仍未关闭：灵感成功 Web/Tauri process、浏览器/代理断线 hash 与重订阅、真实 Tauri 窗口重挂载、其他页面/持久 `start_*` consumer、单次 JSON repair 策略、provider/artifact canonical Host owner、全部持久创建窗口、usage/checkpoint/artifact/cancel-ack 跨进程顺序、`IG-CUTOVER` 以及 CI PostgreSQL 16/Windows/macOS 证据。工作包 09、3.15、3.16、阶段 G 与总体 V2 继续为 `未完成` / `16/44`。

## 批次 261 · Optimize chat canonical generation 收口与测试数据库实跑 · 2026-07-26

状态：`未完成`

本批只迁移 Optimize chat 的真实页面 consumer，复用既有 generation reservation、唯一 client/store/reducer、`raw_json` stream、checkpoint、terminal coordinator 和 queue owner；没有新增同步/流式兼容桥、第二 reducer/store 或测试专用 endpoint：

1. 前端发送和重挂载统一走 `start_optimize_chat_generation` 与 `generationClient.listActive(projectId)`；按 `target.kind=optimization_chat`、项目和 session id 绑定任务，增量预览只消费未重放的 `raw_json` delta。
2. canonical worker 复用 Agent Loop 的 `execute_llm_generation`；多次调用累加 generation usage，单次 `LlmGenerationOutput.usage` 保持 provider call usage，analysis/patch 旧 collector 明确不在本批范围。
3. terminal completion 在同一事务中锁定 session/turn，一次性写正式 user/assistant message、完成 Turn 并返回 `optimization_turn` result ref；前端校验 Turn 所属、completed 状态与 `final_reply` SHA-256 revision，不一致显式进入 `consistency_error`。
4. 删除旧 `send_optimization_message` 生产执行入口、旧 task scope/dispatch/Tauri 注册；保留 `preview_send_optimization_message_request` 作为请求预览合同。
5. 确认项目测试数据库不是缺失：启动 `a4n-postgres` 后，`a4n_test` 可用；当前 migrator 已补齐四个历史版本 `video_workflow_exclusive_claim`、`video_workflow_task_binding`、`neutral_video_segment_plan`、`shooting_script_transition_type_canonicalization`。

验证：

```text
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 60s cargo test -p a4n-db optimize --lib -- --test-threads=1：3/3 通过
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 60s cargo test -p a4n-orchestrator --lib optimize -- --nocapture --test-threads=1：52/52 通过（仅既有 dead-code warning）
cargo test -p a4n-migration --lib migration -- --nocapture：38/38 通过
DATABASE_URL=<Docker a4n_test> cargo run -q -p a4n-migration：数据库迁移完成；四个历史版本均在 seaql_migrations
timeout 120s cargo check -p a4n-api-contract -p a4n-db -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri --no-default-features：通过
npm run check:web-api-parity：通过；generation contract/schema、task-kind boundary：通过
npm run build：通过（2359 modules）
generation::stream 定向单测：2/2 通过
git diff --check：通过；旧生产入口搜索仅剩请求预览命名
```

仍未关闭：Optimize 成功 Web/Tauri process、浏览器/代理断线 hash 与重订阅、真实 Tauri 窗口重挂载、其他页面/持久 `start_*` consumer、单次 JSON repair 策略、provider/artifact canonical Host owner、全部持久创建窗口、usage/checkpoint/artifact/cancel-ack 跨进程顺序、`IG-CUTOVER` 以及 CI PostgreSQL 16/Windows/macOS 证据。工作包 09、3.15、3.16、阶段 G 与总体 V2 继续为 `未完成` / `16/44`。

## 批次 262 · Optimize chat 真实 Server 失败终态进程证据 · 2026-07-26

状态：`未完成`

本批只复用 `cleanup_recovery` 已有隔离 PostgreSQL schema、真实 `a4n-server` 子进程、generation HTTP/SSE endpoint 和 SSE parser，不新增测试 harness、测试专用 endpoint 或第二套事件解析：

1. 新增 Optimize chat process 用例，创建真实用户、文本项目、非空章节、`waiting_user` optimization session 和本地不可达 OpenAI provider，调用 `POST /api/generation-tasks/optimize-chat`。
2. 断言 generation sequence 从 1 连续递增，只有一个且最后一个事件为 `failed`，错误为 `provider_job_failed/provider_connect`；terminal 后 SSE 关闭，summary 状态/result ref/terminal event 与事件流一致。
3. 使用相同 `start_request_id` 重试返回同一个 failed task/attempt，没有创建第二任务；optimization Turn 收口为 `failed`、无 `final_reply` 且保留错误，provider 失败不写正式 user/assistant 消息。

验证：

```text
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 180s cargo test -p a4n-server --test cleanup_recovery server_process_streams_optimize_chat_generation_failure_terminal -- --nocapture --test-threads=1：1/1 通过（runner 3.74s；本次首次增量编译 2m53s）
```

`rustfmt --edition 2021 --check crates/a4n-server/tests/cleanup_recovery.rs` 仍被该共享测试文件既有格式差异阻塞，未用全文件重格式化覆盖其他改动。本批只证明 Optimize Web/Server 失败链路，不冒充成功流式 Web/Tauri process；浏览器/代理断线 hash 与重订阅、真实 Tauri 窗口重挂载、其他页面/持久 `start_*` consumer、provider/artifact canonical Host owner、全部持久创建窗口、usage/checkpoint/artifact/cancel-ack 跨进程顺序、`IG-CUTOVER` 及跨平台 CI 证据仍未完成。工作包 09、3.15、3.16、阶段 G 与总体 V2 继续为 `未完成` / `16/44`。

## 批次 263 · Optimize analysis/patch generation 统一 generation transport · 2026-07-26

状态：`未完成`

本批继续复用唯一 generation reservation/session、`GenerationTaskSession`、`raw_json` stream、checkpoint writer、terminal coordinator、queue projection 和既有 Web/Tauri transport，不新增第二套 progress、SSE parser、recovery client、状态机或测试 endpoint：

1. Optimize analysis 新增 typed generation worker；analysis 正式回复、canonical issues、Turn 状态和 session 状态只在 generation terminal 事务中提交。Optimize patch proposal 同样新增 typed worker，assistant/tool message、patch set、Turn event、session summary 和 Turn completed 在同一事务中提交。
2. analysis、patch、chat 的正式结果统一使用 `result_ref.kind=optimization_turn`；`id` 为正式 Turn ID，`revision` 为 `SHA-256(final_reply)`。前端按 target kind 区分 `optimization_analysis`、`optimization_patch`、`optimization_chat`，patch 只接受 `patch_set.turn_id === result_ref.id` 的正式 patch set。
3. 删除 Optimize 旧 `OptimizeProgress`/`OptimizeLlmStream`、同步分析/补丁执行入口、旧 task scope/dispatch/Tauri 注册和 progress-task 页面依赖；请求预览命令保留为 preview 合同，不作为生产执行旁路。structured repair 继续复用已有 `parse_validate_or_repair_structured_value`，没有新增 repair provider。
4. 补丁准备在创建 running Turn 后，若消息或 Turn event 写入失败，显式调用同一 `fail_turn` 收口为 `failed/cancelled`；不吞失败、不留下孤儿 running Turn。四个历史迁移版本已在 `crates/a4n-migration` 注册，Docker `a4n-postgres` 的 `a4n_test` 实际运行 migrator 成功。当前 migrator 引用的新增 migration source 仍显示为工作区未跟踪文件，提交时必须与 `migrator.rs`/`mod.rs` 一并纳入，否则干净 checkout 会再次报“已应用但文件缺失”。

验证：

```text
A4N_TEST_DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' timeout 120 cargo run -q -p a4n-migration：数据库迁移完成
timeout 120 cargo test -p a4n-db optimize --lib -- --test-threads=1：3/3 通过
timeout 120 cargo test -p a4n-migration --lib migration -- --nocapture：38/38 通过
timeout 180 cargo test -p a4n-orchestrator --lib optimize -- --nocapture --test-threads=1：52/52 通过（仅既有测试 dead-code warning）
timeout 180 cargo test -p a4n-server --test cleanup_recovery -- --nocapture --test-threads=1：7/7 通过
cargo check -p a4n-runtime -p a4n-server -p a4n-tauri --no-default-features：通过
cargo check -p a4n-orchestrator --lib：通过
frontend npm run build：通过（tsc + Vite，2359 modules）
bash scripts/contracts/generate-generation-contracts.sh --check：通过
bash scripts/checks/inventory-generation-paths.sh --check：actual=133、declared=133、missing=0、stale=0
bash scripts/checks/check-api-contract-boundary.sh：通过
bash scripts/checks/check-task-kind-boundary.sh、rustfmt 定向检查、git diff --check：通过
```

本批不关闭 3.15/3.16 或总体 `16/44`：尚无 Optimize 成功 Web/Tauri process、浏览器/代理断线 hash/重订阅、真实 Tauri 窗口重挂载、其他页面/持久 `start_*` consumer、provider/artifact canonical Host owner、全部持久创建窗口、usage/checkpoint/artifact/cancel-ack 跨进程顺序、`IG-CUTOVER` 及 CI PostgreSQL 16/Windows/macOS 证据。当前测试库事实为 `a4n-postgres` healthy、PostgreSQL 16、`a4n_test` 可连接；不要再以“没有测试数据库”解释未执行项。

## 批次 266 · 成功 Web inspiration generation process 与 LLM 队列自锁修复 · 2026-07-26

状态：`未完成`

本批只收口“成功 Web generation process”这一切片，继续复用既有 generation worker、SSE、terminal coordinator 和测试数据库；没有新增 transport、事件桥、recovery facade 或第二队列：

1. 根因是 generation worker 已持有唯一 LLM 队列槽位，`DynamicLlmProvider` 的嵌套 `LlmCall` 又申请同一槽位，导致 worker 等待自身，fake provider 从未收到请求并最终超时。
2. `a4n-queue` 增加 task-local LLM 槽位标记；已持有槽位的嵌套 LLM 任务直接从等待队列取出并复用当前槽位，不释放不存在的额外 semaphore permit。普通 tracked LLM/TextGeneration operation 在取得根 permit 后进入 scope；generation worker 先进入 scope，但根任务强制 non-reentrant 获取真实 permit，之后嵌套调用才能复用。
3. generation worker 在 `AppState::spawn_generation_execution` 进入 LLM scope；成功请求现在真实到达 fake OpenAI-compatible SSE provider，并按现有 canonical stream/terminal 路径完成。

验证：

```text
docker exec a4n-postgres psql -U a4n -d a4n_test -Atc "select current_database(), current_user, version()"：a4n_test / a4n / PostgreSQL 16.14；四个历史 migration 均存在于 seaql_migrations
rustfmt --edition 2024 --check（本批触及 Rust 文件）：通过
timeout 90 cargo test -j 2 -p a4n-queue --lib -- --nocapture：21/21 通过
timeout 180 cargo check -p a4n-server --test cleanup_recovery --features embedded-capabilities：通过
A4N_TEST_DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' timeout 120 cargo test -j 2 -p a4n-server --features embedded-capabilities --test cleanup_recovery server_process_streams_inspiration_generation_success_terminal -- --nocapture：1/1 通过（runner 4.43s）
scripts/checks/refactor-metrics.sh --label "批次 266 完成后" --append：通过；主队列 0/0，AppState 256 文件 / 871 命中
```

本批确认测试数据库不是阻塞项，且成功 Web inspiration process 已通过；不代表整个工作包 09、3.15/3.16 或总体 V2 完成。Tauri 成功 process、断线 hash/重订阅、其他持久 `start_*`、provider/artifact canonical Host owner、持久创建窗口、跨进程 usage/checkpoint/artifact/cancel-ack 顺序、`IG-CUTOVER` 及 CI PostgreSQL 16/Windows/macOS 证据仍未关闭。V2 继续为 `16/44`，工作包 09 保持 `未完成`。

## 批次 267 · temp-media artifact 合同与真实上传/读取 process · 2026-07-26

状态：`未完成`

本批只收口远端临时媒体上传这一条完整生命周期切片，复用已有 Host API contract、文件写入 helper、TTL 清理和 serve 路由，不新增 artifact registry、metadata 表、第二清理 worker 或 provider 删除旁路：

1. 根因是 Runtime 和 Server 各自声明了一套 DTO：Runtime 发送 `owner_user_id/project_id/source_image_id/expires_in_seconds` 并要求 `artifact_id`，Server 没有声明这些字段（serde 默认静默忽略）且响应没有 `artifact_id`，导致远端上传合同实际不可互操作。
2. 新增唯一 `a4n-api-contract::temp_media::{TempMediaUploadRequest, TempMediaUploadResponse}`，请求启用 `deny_unknown_fields` 并携带 owner/audit 范围；响应固定返回 opaque `artifact_id`、HTTPS URL、RFC3339 `expires_at`、owner/project/source、canonical `content_type`、SHA-256 和 `byte_size`。Runtime 校验所有回显字段、内容 hash/大小、artifact id 边界、HTTPS、凭证/fragment 和 24 小时 TTL 上限，不把远端 URL 当成本地路径。
3. Server 上传先校验 owner ID、MIME/魔数、大小、请求 TTL 和公网地址，再使用既有 `write_new_data_file` 写入 `temp-media/video-inputs/<artifact_id>.<ext>`；配置错误或 partial write 不会留下半成品。服务端 TTL 上限固定为 24 小时，默认配置 360 分钟；请求 TTL 低于服务端实际策略时显式拒绝，避免返回的 `expires_at` 与 serve 生命周期不一致。创建日志记录 artifact/owner/audit/大小/过期时间。
4. `cleanup_recovery` 增加真实 Server process：使用 Docker PostgreSQL `a4n_test` 启动真实 `a4n-server`，调用 `/api/temp-media` 上传 PNG，校验返回合同和文件 hash，再从真实 `/api/temp-media/{file_name}` 读取并逐字节比对。

验证：

```text
timeout 240 cargo check -p a4n-api-contract -p a4n-runtime -p a4n-server --tests --no-default-features：通过
timeout 120 cargo test -p a4n-api-contract --lib temp_media -- --nocapture：2/2 通过
timeout 180 cargo test -p a4n-server --bin a4n-server temp_media -- --nocapture：7/7 通过
timeout 180 cargo test -p a4n-runtime --no-default-features --lib remote_temp_media -- --nocapture：2/2 通过（另有既有 `sample_part` dead-code warning）
timeout 180 bash scripts/checks/check-api-contract-boundary.sh：通过
A4N_TEST_DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' timeout 300 cargo test -j 2 -p a4n-server --test cleanup_recovery server_process_returns_canonical_temp_media_artifact_contract -- --nocapture --test-threads=1：1/1 通过（真实 runner 4.08s；编译约 53s）
rustfmt --edition 2024 --check（本批新增/修改的 temp-media、API contract 文件）：通过；`git diff --check`：通过
```

本批关闭了 temp-media 两套 DTO 的合同缺口，但不关闭工作包 09 或 V2：远端 artifact 删除/cancel/status/reconcile 仍没有真实 provider capability owner；generation artifact metadata 尚未接入唯一 canonical snapshot/coordinator；所有持久创建窗口、其他 DB/文件双故障组合、Tauri 成功 process、断线 hash/重订阅、跨进程 usage/checkpoint/artifact/cancel-ack 顺序、`IG-CUTOVER` 和 CI PostgreSQL 16/Windows/macOS 证据仍未完成。总体 V2 保持 `16/44`，工作包 09、3.15、3.16 与阶段 G 继续 `未完成`。

## 批次 268 · Outline 全入口 canonical generation 收口 · 2026-07-26

状态：`未完成`

本批一次迁移 Outline 的全部 10 个生产生成入口，复用现有 generation reservation/session/store/reducer/terminal，不新增第二套任务状态、SSE parser、recovery client 或兼容执行桥：

1. `generate_parts`、`regenerate_parts`、`regenerate_last_part`、`continue_part_outlines`、`regenerate_part_outline`、`generate_part_chapter_outlines`、`batch_generate_part_outlines`、`generate_chapter_outlines_by_count`、`regenerate_chapter_outline`、`apply_beat_mapping` 统一由 `generationClient.start()` 启动，使用唯一 `GenerationTaskHandle`、generation store/reducer/hook 和 canonical cancel。
2. 页面不再生成随机 progress task id，也不再把 `workflow:<task_id>` 作为 Outline 生产状态源；旧同步前端 API 已删除，preview API 保留。`useProgressTask` 只增加独立 `cancelTaskId` 以适配非 workflow transport，没有复制任务状态机。
3. Runtime/Server/Tauri 接入同一个 Outline start/worker；旧同步执行入口、旧 task scope 和 Runtime 纯透传输入文件删除，输入类型直接复用 Orchestrator owner。10 个 operation 使用稳定且唯一的 stage key。
4. provider 配置错误归入 `provider_job_failed`；数据库/序列化持久化失败仍为 `generation_persist_failed`，不再把 provider 配置问题误报为数据库错误。
5. Docker PostgreSQL 16 `a4n_test` 已实际迁移成功；四个历史 migration 均存在。真实 Server process 覆盖 Outline provider 失败、连续 sequence、唯一 terminal、SSE 关闭、summary 一致、start request 幂等以及失败不写正式 `part_outline`。

验证：

```text
generation inventory：actual=127、declared=127、missing=0、stale=0
API contract boundary、generation contract --check、Web API parity：通过；前端缺口均为 0
cargo check -j 2 -p a4n-api-contract -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri --no-default-features：通过
frontend npm run build：通过（tsc + Vite，2359 modules）
a4n-migration migrations：38/38；a4n-db generation::：25/25
Outline stage-key：1/1；Runtime convergence：1/1；Server Outline process：1/1
git diff --check：通过
```

`embedded-capabilities` 全功能 Server 测试首次冷编译在 300 秒内未进入 runner，只能记录为超时，不能写成通过；当前 process 证明 canonical 失败终态，不证明真实 embedded provider 成功。Outline 成功 Web/Tauri process、真实取消/remount、断线重订阅、generation artifact/provider cleanup owner、持久创建窗口和最终 `IG-CUTOVER` 仍未关闭。工作包 09、3.15/3.16、阶段 G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 269 · Portrait canonical generation、artifact 批次与 best-effort 缺失立绘 · 2026-07-26

状态：`未完成`

本批一次收口 Portrait 的四种生产 operation，复用既有 generation reservation/session/worker/client/store/reducer/terminal；没有新增同步兼容入口、第二状态机、第二 SSE parser、第二 recovery client 或第二 cleanup worker：

1. `generate`、`regenerate`、`generate_age_variant`、`generate_missing` 统一接入 `POST /api/generation-tasks/portrait`、Tauri `start_portrait_generation` 和前端 `startPortraitGeneration()`；旧 Server/Tauri/Runtime 同步生产入口及四个前端同步 API 已删除，Galgame 只保留 crate-private 的明确 inline 调用。
2. `GenerationArtifactCoordinator::available` 成为唯一 `ArtifactEvent` 构造入口，事件只携带 opaque `artifact_id`、类型、MIME、SHA-256、大小、revision 和批次 item 元数据，不暴露路径或 URL。完成、失败、取消、中断终态都把已有 artifact 与 terminal event 放进同一 `prepare_committed_events` 批次；partial-cancel 保留已成功 item，不能再把成功 artifact 静默丢掉。
3. 缺失立绘使用稳定 `portrait-profile-<profile_id>` item key 和 `BestEffort` summary；单项 provider/持久化失败后继续后续 profile，精确统计成功、失败、取消、跳过及失败 key；没有缺失项时在 reservation 前显式拒绝零项 batch。
4. 单项与批量 `result_ref.revision` 使用规范字段/成功 ID 集合 SHA-256；前端终态重新读取正式 Portrait 记录并校验 `result_ref` 与 artifact，任何不一致进入唯一 generation store 的 `consistency_error`。Web 等待器把 attach 与 store 订阅并行，避免 Web 直到 SSE 结束才开始显示进度。

验证：

```text
docker exec a4n-postgres psql -U a4n -d a4n_test -Atc "select current_database(), current_user, version()"：a4n_test / a4n / PostgreSQL 16.14；四个历史 migration 均存在
A4N_TEST_DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' A4N_DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' cargo run -q -p a4n-migration：数据库迁移完成
cargo test -p a4n-migration --lib migration -- --nocapture：38/38 通过
scripts/checks/check-api-contract-boundary.sh：通过
scripts/checks/inventory-generation-paths.sh --check：actual=124、declared=124、missing=0、stale=0
scripts/checks/check-task-kind-boundary.sh：通过
timeout 120 cargo check -p a4n-api-contract -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri --no-default-features：通过
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 60 cargo test -p a4n-orchestrator --lib --no-default-features 'portrait::application::tests' -- --nocapture --test-threads=1：5/5 通过
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 60 cargo test -p a4n-orchestrator --lib --no-default-features artifact -- --nocapture：20/20 通过
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 120 cargo test -p a4n-runtime --lib --no-default-features portrait_generation_runtime_uses_application_owner -- --nocapture：1/1 通过（测试 runner 0.01s；测试目标链接约 61s；仅既有 `sample_part` dead-code warning）
timeout 60 cargo check -p a4n-runtime --tests --no-default-features：通过（仅既有 `sample_part` dead-code warning）
frontend npm run test:run -- src/utils/generationEventReducer.test.ts src/utils/sseParser.test.ts src/store/generationTasks.test.ts：3 files / 10 tests 通过
frontend npm run build：通过（tsc + Vite，2359 modules）
frontend npm run check:web-api-parity：通过；所有 Web/桌面/页面直调缺口均为 0
rustfmt --edition 2024 --check（本批触及 Rust 文件）、git diff --check：通过
```

本批不关闭工作包 09、3.15/3.16、阶段 G 或总体 V2。Portrait 的正式文件、portrait/generated_image/镜像等领域持久写入仍发生在 generation terminal 事务之前，当前证据只覆盖 artifact event、terminal event 与 generation checkpoint 的原子批次；其他图片/视频入口、provider cancel/status/delete/reconcile canonical owner、Tauri 成功 process、真实窗口重挂载/断线重订阅、所有持久创建窗口、跨进程 usage/checkpoint/artifact/cancel-ack 顺序和最终 `IG-CUTOVER` 仍未关闭。V2 保持 `16/44`，工作包 09 保持 `未完成`。

## 批次 270 · 五种图片生产入口统一 canonical generation · 2026-07-26

状态：`未完成`

本批一次迁移五种图片生产 operation，复用唯一 generation reservation/session/worker/client/store/reducer/terminal，不新增第二状态机、SSE parser、recovery client 或兼容执行桥：

1. `generate_scene_image`、`generate_page_image`、`batch_generate_images`、`generate_object_profile_image`、`generate_scene_profile_image` 统一使用 `StartImageGenerationInput` / `ImageGenerationOperation`；Server、Tauri、Runtime 和前端旧同步生产入口删除，preview/prompt/query 等非生产 API 保留。
2. 批量请求使用 `BestEffort`，item key 固定为 `image-request-<1-based-index>`；单项失败、取消或跳过不阻塞后续项，summary 精确记录成功/失败/取消/跳过和失败 key，部分成功时保留已提交 artifact。
3. `ArtifactEvent` 由既有 artifact coordinator 构造，不暴露本地路径，只携带 opaque id、MIME、SHA-256、byte size 和 revision；`ImageGenerationHostPort` 增加正式文件 metadata inspection。
4. 六个主要前端 consumer 统一调用 `startImageGeneration()`、`generationClient.waitForTerminal()`、`verifyImageGenerationResult()` 和 canonical `cancel()`；旧 `ImageGenerationTaskScope` 及五类 task scope 名称在生产代码中为 0。

验证：

```text
docker ps/inspect：a4n-postgres running/healthy，PostgreSQL 16.14，端口 5432
docker exec a4n-postgres psql -U a4n -d a4n_test：a4n_test/a4n 可连接；四个历史 migration 均在 seaql_migrations
timeout 120 cargo run -q -p a4n-migration -- 'postgres://a4n:a4n@127.0.0.1:5432/a4n_test'：数据库迁移完成
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 180 cargo test -p a4n-orchestrator --lib image_generation -- --test-threads=1：29/29 通过
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 120 cargo test -p a4n-runtime --lib image_generation --no-default-features -- --test-threads=1：7/7 通过
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 120 cargo test -p a4n-migration --lib migration -- --test-threads=1：38/38 通过
timeout 120 cargo check -p a4n-api-contract -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri --no-default-features：通过
frontend npm run build、npm run check:web-api-parity：通过（2359 modules；parity 缺口 0）
generation inventory：actual=120、declared=120、missing=0、stale=0；API/task-kind/Host DTO boundary、定向 rustfmt、git diff --check：通过
```

当前工作区仍有四个历史 migration 源文件及其他新增 migration 源文件显示为未跟踪；必须与 `migrator.rs`/`migrations/mod.rs` 一并纳入版本库，否则干净 checkout 会再次出现“已应用但迁移文件缺失”。本批不关闭工作包 09、3.15/3.16、阶段 G 或总体 V2：真实成功 Web/Tauri generation process、窗口重挂载/断线 hash、provider cancel/status/delete/reconcile canonical owner、领域持久创建窗口、跨进程 usage/checkpoint/artifact/cancel-ack 顺序和 `IG-CUTOVER` 仍未完成；V2 保持 `16/44`。

## 批次 271 · 图片成功 Web process 与图片队列同槽自锁修复 · 2026-07-26

状态：`未完成`

本批只补通用图片 canonical generation 的成功 Web/Server process 证据，继续复用既有 PostgreSQL 隔离 schema、真实 `a4n-server` 子进程、generation HTTP/SSE、embedded 图片 provider、artifact coordinator 和正式图片持久化，不新增测试 endpoint、第二队列、第二状态机或第二 SSE parser：

1. `cleanup_recovery` 新增 `server_process_streams_image_generation_success_terminal`：真实调用 `POST /api/generation-tasks/image` 的 scene operation，本地 fake OpenAI-compatible provider 返回真实 PNG base64；断言 sequence 从 1 连续递增、只有一个 opaque `generated_image` artifact、MIME/SHA-256/revision/byte size 与真实 PNG 一致且不含路径或 URL，唯一 `completed` 为最后事件，terminal 后 SSE 关闭，summary/result ref/terminal 一致。
2. 数据库只产生一条正式 `generated_image`，artifact id 等于 `generated-image:<id>`，正式落盘文件与 provider PNG 逐字节一致。测试数据库事实为 `a4n-postgres` healthy、`a4n_test`/用户 `a4n` 可连接、PostgreSQL 16.14；四个历史 migration 均存在于 `seaql_migrations`，不能再以“没有测试数据库”解释未执行项。
3. 全量 `embedded-capabilities` 首次冷编译 360 秒内未进入 runner，缓存后又因本机缺少可链接的 ONNX Runtime 在 `OrtGetApiBase` 处失败；两次都不算测试通过。Server 增加窄 `embedded-media-providers` feature，只复用 Runtime 现有 migration 和 embedded 图片/视频 provider，不拉入 FastEmbed/ORT。
4. 窄 feature 首次真实 runner 进入后暴露根因：canonical image task 已持有唯一图片队列槽，Runtime Host 内部又提交第二个 `ImageGeneration` 并等待同一槽，数据库表现为外层 task `running`、内部 task `pending`。复用批次 266 的 task-local 重入模式增加 image slot scope；image/portrait 两个同类 generation worker 使用图片 scope，嵌套 provider task 复用已持有槽位，不伪造或额外释放 semaphore permit。

验证：

```text
docker ps / pg_isready / psql：a4n-postgres healthy；a4n_test accepting connections；四个历史 migration 均存在
timeout 60 cargo test -p a4n-queue --lib -- --test-threads=1：21/21 通过
A4N_TEST_DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' DATABASE_URL='postgres://a4n:a4n@127.0.0.1:5432/a4n_test' timeout 180 cargo test -j 2 -p a4n-server --features embedded-media-providers --test cleanup_recovery server_process_streams_image_generation_success_terminal -- --nocapture --test-threads=1：1/1 通过（最终 runner 3.92s）
scripts/checks/inventory-generation-paths.sh --check：actual=120、declared=120、missing=0、stale=0
scripts/checks/check-api-contract-boundary.sh、scripts/checks/check-task-kind-boundary.sh：通过
本批触及 Rust 文件 rustfmt --check、全树 git diff --check：通过
```

本批关闭了通用图片成功 Web process 证据和 image/portrait 同槽自锁，但不关闭工作包 09、3.15/3.16、阶段 G 或总体 V2。Tauri 成功 process、真实窗口重挂载/代理断线 hash、provider cancel/status/delete/reconcile、领域文件/数据库与 generation terminal 的持久创建窗口、跨进程 usage/checkpoint/artifact/cancel-ack 顺序、`IG-CUTOVER` 与 CI PostgreSQL 16/Windows/macOS 证据仍未完成；V2 保持 `16/44`。新增 migration source 仍需与注册表一起纳入版本库，避免干净 checkout 再次出现“已应用但迁移文件缺失”。

## 批次 272 · Tauri Channel 活动任务重挂载与 replay 取消收口 · 2026-07-26

状态：`未完成`

本批只补 Tauri generation transport 的可执行本地证据，复用生产 command、唯一 subscription registry、canonical image worker、既有 PostgreSQL 隔离 schema 与共享 `FakeJsonProvider`；没有新增测试专用 endpoint、第二 transport、第二 reducer/store、第二 provider 或窗口私有状态机：

1. `subscribe_generation_task` / `unsubscribe_generation_task` 改为对 `tauri::Runtime` 泛型，生产 Wry 继续由原 command registry 推断，测试则可让 Tauri 官方 `MockRuntime` 调用同一 command；没有复制一份测试 command。
2. `pump_subscription` 在初始 replay 和 live replay 的每个事件发送前检查窗口 cancellation。此前窗口 cleanup 若发生在一个已取出的 replay 批次中，仍会把该批剩余事件继续送到旧窗口；现在首个事件触发 cleanup 后立即停止，第二窗口可按最后确认 cursor 继续订阅。
3. 新用例通过 Tauri `get_ipc_response` 实际调用生产 `start_image_generation`、`subscribe_generation_task` 和 `get_generation_task_summary`。真实 Docker `a4n_test` 中创建用户/项目/provider 配置，延迟 fake OpenAI-compatible image provider 保持任务 active；首窗口收到 `sequence=1` 后执行同一 `GenerationSubscriptionRegistry::cancel_window` cleanup，第二个 MockRuntime window 从 `after_sequence=1` 重挂载并收到连续 sequence、唯一 opaque artifact 和唯一最终 completed。
4. completed 的 result ref、summary、terminal event、artifact MIME/SHA-256/revision/byte size、唯一 `generated_image` 记录和正式落盘 PNG 字节全部一致；批次 271 的真实 Server/SSE 图片成功 process 同时复验通过。测试数据库不是阻塞：`a4n-postgres` healthy，`a4n_test` 为 PostgreSQL 16.14，四个历史 migration 均存在于 `seaql_migrations`。
5. `a4n-migration::test_support` 只增加“既有隔离 schema connection + 对应 search_path URL”的共享返回 helper；`FakeJsonProvider` 只增加可选延迟入口，普通 `start` 仍复用零延迟实现。没有新建数据库 harness 或图片 provider。

验证：

```text
docker ps / inspect / psql：a4n-postgres healthy；a4n_test / a4n / PostgreSQL 16.14；四个历史 migration 均存在
首次 Tauri 窄 feature 冷编译 timeout 180：退出码 124，未进入 runner，不计为通过
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 120 cargo test -j 2 -p a4n-tauri --features embedded-media-providers tauri_channel_remounts_active_image_generation_after_window_cleanup -- --nocapture --test-threads=1：1/1 通过（runner 4.06s）
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 60 cargo test -j 2 -p a4n-tauri --features embedded-media-providers -- --test-threads=1：26/26 通过（runner 3.17s）
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16 a4n_test> timeout 180 cargo test -j 2 -p a4n-server --features embedded-media-providers --test cleanup_recovery server_process_streams_image_generation_success_terminal -- --nocapture --test-threads=1：1/1 通过（runner 3.05s）
timeout 60 cargo test -j 2 -p a4n-test-support --lib -- --test-threads=1：35/35 通过
timeout 120 cargo check -j 2 -p a4n-tauri --no-default-features：通过（最终缓存复验 51.76s；此前两次 60s 均只停在检查阶段，不计为通过）
generation inventory：actual=120、declared=120、missing=0、stale=0
API contract boundary、generation contracts、TaskKind boundary、定向 rustfmt、git diff --check：通过
```

Tauri 官方 `MockRuntime` 的 window `close()` / event loop 是 no-op，因此本批只能关闭“生产 command + MockRuntime IPC/Channel + registry cleanup + active cursor 重挂载”的本地证据，不能写成真实 Wry 桌面 process 或真实 `WindowEvent::Destroyed` process。真实 Wry/Destroyed、浏览器/代理断线与 1MB hash、provider cancel/status/delete/reconcile canonical Host owner、领域持久创建窗口、跨进程 usage/checkpoint/artifact/cancel-ack、`IG-CUTOVER` 和 CI PostgreSQL 16/Windows/macOS 仍未关闭；工作包 09、3.15/3.16、阶段 G 与总体 V2 保持 `未完成` / `16/44`。新增 migration source 仍需与 `migrator.rs`/`migrations/mod.rs` 一起纳入版本库。

## 批次 273 · Server SSE 十次断线重连、1 MiB hash 与文本事件分片收口 · 2026-07-26

状态：`未完成`

本批复用既有 PostgreSQL 隔离 schema、真实 `a4n-server` 子进程、生产 inspiration start/events/summary endpoint、canonical generation session/replay、同一个 SSE parser 和延迟 fake OpenAI SSE provider，没有新增测试 endpoint、第二 parser、第二状态机、recovery client 或 provider facade：

1. `cleanup_recovery` 的灵感成功 process 升级为 1,048,576 字节回复；HTTP SSE 客户端在任务仍 active 时每次只读取一个非终态事件，主动丢弃响应共 10 次，再以 `Last-Event-ID=1:<sequence>` 继续订阅，最后读取至终态。
2. 全部事件从 sequence 1 连续递增，无重复、无缺失；只有一个 usage 和一个最终 completed。SSE `raw_json`、completed `final_text_hashes`、`result_ref.revision`、summary、正式 inspiration assistant message 与数据库权威行为一致；reply SHA-256 为 `8f990ba0b577b51cf009ea049368c16bbda1b21e1b93be07a824758bb253c39b`。
3. 首次 1 MiB runner 在约 67.10 秒出现 `Body TimedOut`。根因是共享 stream 自定义 512-byte batching 产生约 2,048 个 text event，session 对不断增长的 snapshot/replay 反复维护。直接删除该自定义阈值，`BATCH_BYTES` 复用协议已有 `MAX_TEXT_DELTA_BYTES`（16 KiB），50ms 实时 flush 保留；最终总事件数降为 76，定向 runner 约 10.52 秒。
4. 完整窄 feature 首轮为 9/10：Outline provider 连接失败错误地得到 `generation_persist_failed`。共享 `a4n-llm-http::request_error` 将非超时 reqwest 请求失败包装为 `AppError::Internal`，使领域映射误判为持久化失败；唯一根因修复是在 HTTP LLM adapter 改为 `AppError::LlmError`，未在 Outline 增加字符串匹配或放宽所有 Internal 错误。
5. provider cleanup owner 审计确认 typed outbox、bounded retry 与 terminal 同事务 intent 已存在，但 canonical video generation 尚未持久化完整 provider identity，现有 sidecar contract 也没有 cancel/delete。为避免猜配置并制造半套 owner，本批不新增临时 registry/worker，该关闭条件继续保留。

验证：

```text
docker ps / psql：a4n-postgres healthy；a4n_test / a4n / PostgreSQL 16.14；四个历史 migration version 均在 seaql_migrations
首次 1 MiB process：Body TimedOut（runner 67.10s），不计为通过；修复 16 KiB batching 后最终定向 process 1/1（runner 10.52s，events=76）
timeout 60 cargo test -j 2 -p a4n-llm-http --lib -- --test-threads=1：28/28 通过
A4N_TEST_DATABASE_URL=<Docker a4n_test> timeout 180 cargo test -j 2 -p a4n-server --features embedded-llm-provider --test cleanup_recovery server_process_streams_outline_generation_failure_terminal -- --nocapture --test-threads=1：1/1 通过（runner 4.03s）
A4N_TEST_DATABASE_URL=<Docker a4n_test> timeout 180 cargo test -j 2 -p a4n-server --features embedded-llm-provider --test cleanup_recovery -- --nocapture --test-threads=1：10/10 通过（runner 37.88s）
generation inventory：actual=120、declared=120、missing=0、stale=0；generation/API/TaskKind boundary、定向 rustfmt、git diff --check：通过
```

共享 stream batching 的 Unicode 边界单测在本批修改后已实际 2/2 通过；最终格式化后的三次 60 秒复验均停在 Orchestrator 聚合测试二进制重新链接、未进入 runner，因此不把这些复验写成新增通过。完整 Server process 已实际执行同一 batching 路径并 10/10 通过。

本批关闭的是“真实 Server HTTP SSE 客户端主动断线 10 次 + 1 MiB hash”的 process 证据，不冒充真实浏览器、外部代理或桌面 Wry。真实 Wry/`WindowEvent::Destroyed`、真实浏览器/代理网络中断、provider cancel/status/delete/reconcile canonical Host owner、领域持久创建窗口、跨进程完整 usage/checkpoint/artifact/cancel-ack 顺序、`IG-CUTOVER` 与跨平台 CI 仍未关闭；工作包 09、3.15/3.16、阶段 G 与总体 V2 保持 `未完成` / `16/44`。migration source 仍需与注册表一起纳入版本库，避免干净 checkout 再次出现“已应用但迁移文件缺失”。

## 批次 274 · Remote staging lifecycle、provider 幂等与重启 DELETE · 2026-07-27

状态：`未完成`

本批只收口 canonical video provider 的 remote staging 和请求幂等链，复用已有 generation checkpoint、cleanup outbox、worker、provider trait 和 sidecar protocol；没有新增 artifact registry、cleanup worker、状态机、recovery client 或兼容协议：

1. remote-temp 上传不再只返回 URL。Runtime 保留私有 `GenerationArtifactRef`，包含 opaque `artifact_id`、`artifact_kind=remote_temp_media`、MIME、SHA-256、byte size、revision、`temporary=true` 和 `expires_at_ms`；只写入 `execution_checkpoint_json.staging_artifacts`，不进入 public snapshot、不发送 staging artifact event，也不保存 URL 或本地路径。
2. provider request checkpoint 与 provider job checkpoint 都保留 staging metadata。generation terminal 在同一事务中复用既有 `generation_staging_artifact_delete` intent；未做 durable bind 的 staging 输入不插入无意义的 artifact unbind，同一 artifact 按既有 dedupe key 只生成一条 delete。
3. canonical provider request checkpoint 使用现有 RFC 8785 `canonical_json_sha256`，对 task/attempt/video/provider/model/request 生成幂等键。该键经 `VideoProviderRequestInput` 贯通 embedded 与 sidecar，六个 provider 共用唯一校验 helper 写入 HTTP `Idempotency-Key`；Canvas 非 canonical 调用显式传 `None`，不使用纯请求 hash 跨任务去重。
4. sidecar payload 新字段与旧 v3 严格反序列化不兼容，因此 capability 明确从 `video-provider-v3` 升级到 `video-provider-v4`；没有伪装兼容、v3 fallback 或双协议分支。
5. 真实 PostgreSQL/Server process 在 owner-lost 恢复时生成唯一 interrupted terminal、两条 provider cleanup 和一条 staging delete。loopback remote Host 实际收到正确 artifact id、SHA-256 和 Bearer token，cleanup 行收口为 `completed/deleted`；停止并再次启动 Server 后没有第二次 DELETE，也没有重复插入 intent。

验证：

```text
cargo test -p a4n-video provider_idempotency_key_is_optional_lowercase_sha256：1/1 通过
cargo test -p a4n-video-providers adds_validated_idempotency_header：1/1 通过；补齐该 crate 既有 tokio 测试缺失的 workspace dev-dependency
cargo test -p a4n-runtime --no-default-features video_generation_maps_workflow_run_to_service_trace：1/1 通过
A4N_TEST_DATABASE_URL=<Docker a4n_test> cargo test -p a4n-tauri --features embedded-media-providers tauri_channel_remounts_active_image_generation_after_window_cleanup：1/1 通过（runner 4.26s）
A4N_TEST_DATABASE_URL=<Docker a4n_test> cargo test -p a4n-server --test cleanup_recovery server_process_recovers_snapshot_only_generation_subscription：1/1 通过（runner 4.44s）
cargo check -j 2 -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri：通过（默认 feature，2m43s）
generation inventory 120/120、API boundary、TaskKind boundary、generation contracts、sidecar current/historical/rollback 四向矩阵、release manifest 与 git diff --check：通过
frontend npm run build：通过（tsc + Vite，2360 modules）
```

本批没有伪造 provider cancel/delete：当前 capability 仍是 `cancel=false`、`delete=false`，异步 provider 只对真实支持的 status query/reconcile 执行；unsupported 是显式结果。Canvas 非 canonical remote-temp 上传仍只依赖远端 24 小时 TTL。真实 Wry/`WindowEvent::Destroyed`、真实浏览器/外部代理、所有领域持久创建窗口、跨进程完整 progress/checkpoint/artifact/usage/cancel-ack/terminal 组合、`IG-CUTOVER` 与跨平台 CI 仍未关闭；工作包 09、3.15/3.16、阶段 G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 275 · 旧事件符号清零与 capability/Host 边界裁决 · 2026-07-27

状态：`未完成`

本批执行 IG-CUTOVER 的 Rust 旧合同清零，但不把 provider 原始 transport 伪装成 generation domain event：

1. 全量删除 `ServiceTaskEvent/Kind`、`SseEvent`、`ProgressTaskKind`、`event::sse` 和两个旧模块文件，不保留 type alias、兼容 re-export 或双协议分支。Sidecar/Runtime 私有传输统一为 `CapabilityTaskEvent/Kind`；Host 后台任务事件明确为 `HostTaskEvent`。
2. `ProgressMeta.task_kind` 直接写 canonical `TaskKind::as_str()`，删除重复展示 enum；现有 JSON 字段和枚举值保持不变，不制造无 wire 变化的 protocol version 升级。
3. ADR-IG-001 固定边界：capability transport 可承载 provider 原始 payload、usage 和 artifact location，但不得进入 generation replay/Host API/前端；只有 Runtime 取得真实 Host task UUID/attempt/offset 后才能发布唯一 `GenerationTaskEvent`。
4. 新增 `check-generation-cutover-boundary.sh` 并接入 `module-checks.yml`，同时阻止旧符号、旧模块路径、alias/re-export 和旧文件复活。inventory 扫描规则未放宽；共享工作树已完成的前端/完整响应迁移同步删除 stale machine keys，当前为 `46/46`。

验证：

```text
cargo check -j 2（core/queue/protocol/capabilities/bootstrap、五个 sidecar、test-support、Orchestrator/Runtime/Server/Tauri，共 16 crates）：通过（5m46s）
a4n-capabilities 26/26、a4n-service-protocol 69/69、a4n-sidecar-bootstrap 19/19、a4n-test-support 35/35、a4n-core event/channel 8/8：通过
a4n-server service_and_server_preserve_progress_terminal_order：1/1 通过
a4n-tauri service_and_tauri_preserve_progress_terminal_order：300 秒仍在 GTK/WebKit 测试依赖编译，未进入 runner，不计为通过
generation cutover boundary、generation inventory 46/46、API/generation contract、TaskKind boundary、sidecar current/historical/rollback 四向矩阵、release manifest、定向 rustfmt、git diff --check：通过
check-refactor-docs.sh：失败；`docs/重构/模块化重构执行计划-v2.md` 尚未被 Git 跟踪
```

本批只关闭旧 Rust 事件符号和重复 task kind 展示 enum，不关闭完整 IG-CUTOVER：`workflow:{task_id}` Host channel、剩余 46 个 generation-path machine keys、生产完整响应/structured stream、真实 Wry/浏览器/外部代理、所有领域持久创建窗口、完整跨进程事件组合与跨平台 CI 仍需收口。新 `host_task.rs`、capability events 模块、cutover gate 和整个 `docs/重构` 仍显示为未跟踪；后续提交必须与模块声明/CI 同批纳入，否则干净 checkout 会缺文件。工作包 09、3.15/3.16、阶段 G 和总体 V2 保持 `未完成` / `16/44`。

> 上段是批次 275 当时的中间态；批次 276 已删除其中的 Host task channel、`host_task.rs` 和对应 bridge，当前状态以下段为准。

## 批次 276 · 删除无消费者 Host task bridge 与空 Web SSE · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 删除没有真实消费者的第二事件合同，不新增替代 enum、publisher、worker 或兼容壳：

1. 删除 `HostTaskEvent`/`host_task.rs`、Tauri `EventBridge`、Server task event publisher、`WebEventHub` 和通用 `/events/{channel}`。前端从未订阅 `host-task:*`；通用 Web endpoint 没有任何 producer，只会连接后永久空转。
2. `TaskHandle` 删除 `terminal_event` 和全部 event 构造方法，只保留 cancellation token、`TaskProgress` 快照和有界进度历史。Server/Tauri 成功或失败后立即移除 handle；canonical generation 的 durable `terminal_event` 完全独立且未改动。
3. `WorkflowProgressTask` 从 task id/bridge/重复 cancel token/terminal 方法缩成 `TaskHandle + TaskKind`；删除只负责转发 chunk/progress 的 worker和无用 Window 注入。Embedding、Manga、Galgame、export、RAG、chapter、character 和 video prompt 只保留真实快照/取消/持久 workflow 行为。
4. 仍有用途的 `ProgressMeta`/phase/severity 从事件模块迁到 `event::progress`，没有保留 `event::host_task` re-export。Manga 后台错误改为显式结构化日志，未吞错。
5. `listenAppEvent` 收敛为原生 Tauri domain notification；三个 `chapter-analysis-updated` 页面通过 runtime capability 判断只在真实 Tauri 模式监听，Web 继续依靠命令结果与 focus refresh，不再连接无生产者 SSE。
6. cutover 门禁新增已删除文件、符号、module path、channel、publisher 和通用 endpoint 检查；当前生产/测试源码相关引用为 0。

验证：

```text
cargo check -j 2 -p a4n-core -p a4n-queue -p a4n-db -p a4n-runtime -p a4n-server -p a4n-tauri：通过，无 warning
cargo test -p a4n-core event::channel：6/6 通过
frontend npm run build：通过（tsc + Vite，2360 modules）
frontend npm run check:web-api-parity：API/generation contract、Tauri/Web mapping 和 runtime boundary 全部通过
check-generation-cutover-boundary.sh：通过；Host event/bridge/hub 引用 0
git diff --check（本批路径）：通过
cargo test -p a4n-server routes::commands::task：三次 60 秒均停在依赖测试编译，未进入 runner，不计为通过或失败
定向 rustfmt --check：目标模块本身通过；从 routes 根递归检查时发现共享工作树 temp_media/delete.rs 的既有格式差异，本批未改该文件
```

Generation inventory 扫描规则未放宽；当前 machine block 仍为 `46/46`（需以 `inventory-generation-paths.sh --check` 最终复验）。本批关闭的是无消费者 Host bridge，不是发布级 IG-CUTOVER 或工作包 09：真实 Wry/浏览器/外部代理、provider cancel/status/delete/reconcile capability、所有持久创建窗口、完整跨进程 canonical generation 事件组合和跨平台 CI 仍未关闭。3.15、3.16、阶段 G、工作包 09 与总体 V2 保持 `未完成` / `16/44`。

## 批次 277 · 本地持久创建强杀窗口与 DB/文件双向失败证据 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 复用现有六根受管媒体 scanner、orphan project recovery、cleanup outbox 和数据库事务，没有新增 created-file registry、pre-create intent、worker 或状态机：

1. 全量复扫当前 32 个直接文件 I/O 生产文件和两个 package Host port。本地持久创建只落在 `images/videos/portraits/audio/media/assets` 六个受管根或尚未提交的新项目目录；前者由启动 scanner + canonical DB 引用复核恢复，后者由 orphan project recovery 恢复。上传临时文件和 assembly cache 继续分别使用 24 小时/7 天 TTL。
2. 新增真实 Server process 用例：PostgreSQL advisory transaction lock 在 `generated_image` INSERT 内阻塞请求，等待图片真实落盘后强杀第一台 Server。此时 DB 没有 generated image 或 cleanup intent；释放锁并重启后，唯一 startup scanner 写入既有 outbox，worker 以 attempt 1 删除 orphan 并完成 intent。
3. 新增真实 PostgreSQL package import 用例：事务先插入新项目和 pending media row，现有 `ProjectFamilyMediaFileImporter` 返回文件写失败后，事务退出并确认项目/media rows 均为 0，覆盖 DB 成功步骤之后的文件失败组合。
4. 修正 canonical media reference 测试的过期角色属性夹具；用例重新证明删除前 DB 复核覆盖 generated image、video/thumbnail、portrait 和 media asset。

验证：

```text
PostgreSQL 16.14 / a4n_test：已确认真实连接
Server 新增强杀窗口 process：1/1 通过
Server recover 族串行：4/4 通过；并行运行因套件的单 generation Host owner 合同失败，不是生产回归
a4n-db cleanup_outbox：16/16 通过
a4n-db canonical media reference：1/1 通过
a4n-db 文件 writer 失败事务回滚：1/1 通过
Runtime file_utils：三次 60 秒均停在聚合测试二进制编译/链接，未进入 runner，不计为通过或失败
目标 Rust 文件 rustfmt --check、git diff --check：通过
```

本批关闭“所有本地持久创建窗口仍无证据”和 DB/文件双向失败组合缺口，但不伪造外部 provider 能力：当前 video capability 仍明确为 `cancel=false`、`delete=false`，远端临时 artifact 仍依赖服务 TTL/已有 staging delete 合同。3.15、阶段 F、3.16、阶段 G、工作包 09 与总体 V2 保持 `未完成` / `16/44`。

## 批次 278 · Tauri Channel cancel ack 与唯一终态 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 只扩展既有 Tauri generation process 测试，复用同一 AppState、PostgreSQL、图片 provider、IPC command、Channel、session 和 summary，没有新增 transport、事件类型、recovery client 或测试框架：

1. 现有测试先保持成功图片生成、首窗口 cleanup、第二窗口按 cursor 重挂载、opaque artifact、唯一 completed 和正式文件一致性。
2. 同一 process 随后通过生产 `start_image_generation`、`subscribe_generation_task` 和 `cancel_generation_task` 启动第二个任务；Channel sequence 连续，`cancel_requested` 紧邻唯一 `cancelled` 终态，terminal 后不再发送事件。
3. `get_generation_task_summary` 返回 `cancelled`，且 `terminal_event` 与 Channel 最后一项完全相同，证明 Tauri adapter 没有重建 cancel ack 或终态。

验证：

```text
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16.14 a4n_test> cargo test -p a4n-tauri --features embedded-media-providers tauri_channel_remounts_active_image_generation_after_window_cleanup -- --nocapture --test-threads=1：1/1 通过（runner 4.10s）
目标 rustfmt、git diff --check：通过
批次 277 联合 cargo check：60 秒检查到 a4n-runtime 后超时，未完成 a4n-server --tests，不计为通过或失败
```

本批只关闭 Tauri MockRuntime Channel 的 cancel ack/唯一终态切片。官方 MockRuntime 不产生真实 Wry event loop 或 `WindowEvent::Destroyed`；真实浏览器/外部代理、完整跨进程 usage/checkpoint/artifact/cancel-ack 组合、provider 实际 delete/cancel、远端 artifact 生命周期、发布级 `IG-CUTOVER` 与跨平台 CI 仍缺。工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 279 · 真实 Chromium 代理断线与 Web cursor 恢复 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 复用生产 generation client、现有 Server process runner、真实 PostgreSQL 和系统 Chromium/ChromeDriver，不新增 SSE parser、transport、recovery 状态机或浏览器测试依赖：

1. 修复 Web fetch-SSE 的共享根因：正常 EOF 原本会重连，但浏览器在代理断链时由 `fetch`/`reader.read()` 抛出的网络 `TypeError` 会直接进入 recovery failed。现在仅该网络错误沿既有 backoff 重连，并携带最后已验证 sequence 的 `Last-Event-ID`；主动 abort、HTTP 业务错误、schema/协议错误继续显式失败。
2. generation schema 实际使用 `int32/int64/uint16/uint32/uint64`，严格 Ajv 注册表漏掉 `int32` 会在生产 client 加载时失败；补齐既有 format 列表，不添加自定义 validator。
3. 新增一个前端回归：首个完整 SSE 后 stream 抛 `TypeError`，第二次请求必须携带 `Last-Event-ID=1:1` 并到达 completed。
4. 真实 process 使用 PostgreSQL 16.14、`a4n-server`、延迟 fake OpenAI SSE provider、Vite 编译的生产前端 client、真实 Chromium 和独立 Axum 代理。代理确认客户端消费首个完整 SSE frame 后注入 connection reset；浏览器自动重连并到唯一 completed，sequence、usage、正文 byte size、summary terminal 和 result revision 一致。

验证：

```text
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16.14 a4n_test> cargo test -p a4n-server --features embedded-llm-provider --test cleanup_recovery real_chromium_recovers_generation_after_proxy_aborts_sse -- --nocapture --test-threads=1：1/1 通过（runner 9.35s）
frontend generationTasks/sseParser/store：3 files、6/6 通过
frontend npm run build：通过（tsc + Vite，2360 modules）
frontend npm run check:web-api-parity：通过
generation inventory：actual=46 / declared=46 / missing=0 / stale=0
generation cutover boundary、目标 rustfmt、git diff --check：通过
```

本批关闭真实 Chromium + 独立本地代理的 Web 网络断线/cursor 恢复切片，不冒充真实 Wry `WindowEvent::Destroyed`、远端部署代理或完整跨进程 usage/checkpoint/artifact/cancel-ack 单组合。provider 的实际 capability 仍为 `cancel=false`、`delete=false`，远端 artifact 生命周期、发布级 `IG-CUTOVER` 与跨平台 CI 仍缺。工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 280 · 同一真实浏览器 process 的完整事件族组合 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 扩展批次 279 的唯一 Chromium/Server/PostgreSQL runner，不增加 transport、事件类型、SSE parser、provider facade 或浏览器依赖：

1. 同一真实浏览器依次运行三个生产任务：inspiration 成功流提供 cumulative usage 和正式 result revision；scene image 成功流提供唯一 opaque artifact 和正式 generated-image result revision；第二个 inspiration 经真实 cancel REST 后提供 cancel ack/唯一 cancelled 终态。
2. 三个任务都由生产 `generationClient` 消费 canonical Server SSE；public snapshot 的 `covers_through_sequence` 必须等于客户端最终 sequence，summary terminal sequence 必须一致。文本 usage 为 5；图片 artifact revision 校验真实 PNG SHA-256，terminal result ref 与 summary result ref 一致；取消任务至少包含 started、cancel_requested、cancelled 三个 sequence。
3. 图片 artifact revision 是文件 SHA-256，`result_ref.revision` 是正式 `generated_image` 行字段 hash；测试分别校验，不再错误地把两个独立 revision 强行视为同一个值。
4. 同时审计 Tauri 2.11 本机源码：`MockRuntime::on_window_event` 只分配 listener id，不保存或派发窗口事件；真实 `Destroyed` 只来自 Tao/Wry event loop。现有生产 `on_window_event → handle_window_destroyed` 入口保留，未添加测试专用伪事件 API。

验证：

```text
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16.14 a4n_test> cargo test -j 2 -p a4n-server --features embedded-llm-provider,embedded-media-providers --test cleanup_recovery real_chromium_recovers_generation_after_proxy_aborts_sse -- --nocapture --test-threads=1：1/1 通过（最终 runner 7.25s；首次双 feature 编译 1m49s）
frontend generationTasks/sseParser/store：3 files、6/6 通过
frontend npm run build：通过（tsc + Vite，2360 modules）
frontend npm run check:web-api-parity：通过
generation inventory：actual=46 / declared=46 / missing=0 / stale=0
generation cutover boundary、目标 rustfmt、git diff --check：通过
```

本批关闭本地真实浏览器/Server/PostgreSQL 的 usage、durable checkpoint、artifact、cancel ack 和唯一终态单 process 组合；结合批次 278 的 Tauri Channel 顺序证据，本地两种 Host transport 均直接消费同一 canonical generation owner。仍未关闭真实 Wry `WindowEvent::Destroyed`、远端 provider 的真实 cancel/delete、远端 artifact 生命周期、发布级 `IG-CUTOVER` 与 Windows/macOS CI。工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 281 · IG-CUTOVER 完整响应 LLM 入口清零 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 删除已无生产消费者的完整响应 LLM 合同，不增加兼容层、第二 transport、worker、registry 或 parser：

1. 删除 `LlmProvider::chat`、`LlmClient::chat`、HTTP provider/client 非流式方法、`/llm/chat` route，以及 `http/provider/chat.rs`、`handlers/chat.rs` 两个实现文件。完整响应统一由既有 `collect_chat_stream` 从唯一 stream owner 收集。
2. 测试 provider、recording client 和 gateway process 全部改走真实 stream；probe 与完整结果收集也验证为 `stream=true`。版本不匹配沿 `/llm/stream` 的 canonical `CapabilityTaskEvent::Failed` 合同断言，不为单一消费者扩公共测试 helper。
3. 该公开 capability breaking change 只把 LLM 从 `llm-chat-v2` 升为 `llm-chat-v3`；其他七个 sidecar 无 wire 变化，全局 `PROTOCOL_VERSION` 仍为 2。cutover gate 禁止旧文件、route 和 trait 方法复活。
4. gateway 单测 Router 的认证层改在 owning crate 以 `#[cfg(not(test))]` 安装，修正依赖 crate 中 `cfg!(test)` 不生效的根因；独立 process 和生产 binary 仍严格要求 sidecar auth token，没有放宽生产认证。

验证：

```text
a4n-capabilities：26/26
a4n-llm：46/46
a4n-llm-gateway unit：19/19，另 1 个手工 stage-0 baseline ignored
a4n-llm-gateway process：11/11
a4n-test-support：35/35
a4n-llm-http：28/28
cargo check -j 2 -p a4n-orchestrator -p a4n-runtime -p a4n-server -p a4n-tauri：通过（首次 60 秒到 server/tauri 后超时，热缓存复验 44.64 秒通过）
generation cutover boundary、sidecar 四向真实历史版本矩阵、release manifest 防篡改、目标 git diff --check：通过
旧入口全量搜索：除 cutover gate 自身禁止正则外为 0
```

本批只关闭 IG-CUTOVER 的“生产完整响应 `chat()`/`chat_llm()` 清零”切片，不宣称整个发布级切换完成。真实 Wry `WindowEvent::Destroyed`、远端 provider 实际 cancel/delete、远端 artifact 生命周期和 Windows/macOS CI 仍缺；工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 282 · OpenAI Video provider 真实 DELETE 与 cleanup outbox 闭环 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 复用唯一 video capability registry、现有 provider/sidecar client 和 cleanup outbox，不新增 transport、worker、registry、状态机、parser 或 fallback：

1. 官方 OpenAI Videos API 提供 `DELETE /v1/videos/{video_id}`，因此仅 `cliproxy-openai-video` 声明 `delete=true` 并实现 Bearer DELETE；2xx 映射 `Deleted`，404 映射幂等 `NotFound`，其他状态显式失败。Google Gemini Veo operations 只有 get/list；Gemini、NewAPI、Seedance、generic compatible delete 和所有 provider cancel 继续显式 unsupported，没有伪造 capability。
2. `VideoProvider`、`VideoClient`、video sidecar、Runtime embedded/sidecar routing 共用一个 typed `VideoJobDeleteOutcome`；新增严格 `/video/delete` 合同后 capability 从 `video-provider-v4` 升为 `video-provider-v5`，全局 protocol 仍为 2，不保留 v4 fallback。
3. cleanup outbox 的 Delete 分支先查唯一 capability registry；不支持的 backend 直接完成 `unsupported`，支持的 backend 复用既有 provider config identity、续租和 provider adapter。旧 fallback 不再把 provider cleanup 静默完成为 unsupported。
4. 真实 Docker PostgreSQL 16.14 + `a4n-server` owner-lost 恢复 process 从已持久化 video config/record 执行 status → `delete_scheduled` → loopback `DELETE /openai/v1/videos/provider-job-owner-lost`，校验 Bearer、`completed/deleted`，Server 重启后没有第二次 DELETE 或重复 intent。

验证：

```text
a4n-video + a4n-video-providers + a4n-test-support：98/98
a4n-runtime video/cleanup 定向：11/11 + 4/4
a4n-video-service unit/process：12/12 + 1/1
A4N_TEST_DATABASE_URL=<Docker PostgreSQL 16.14 a4n_test> cargo test -p a4n-server --features embedded-media-providers --test cleanup_recovery server_process_recovers_snapshot_only_generation_subscription：1/1（runner 4.96s）
cargo build --workspace --locked -j 6：通过；四 Host cargo check 三次各 60 秒只在 Tauri/GTK 冷依赖检查中超时，不计为通过
sidecar current/legacy/current/rollback 四向真实历史版本矩阵：通过
release manifest、sidecar package inventory、目标 rustfmt、git diff --check：通过
```

本批只关闭 OpenAI Videos DELETE 与本地 cleanup 恢复闭环。未调用远端 paid provider，不宣称 Gemini/其他 compatible backend 可删除；所有 provider cancel、其他 backend delete、真实 Wry `WindowEvent::Destroyed`、远端 artifact 生命周期、发布级完整 `IG-CUTOVER` 和 Windows/macOS CI 仍缺。工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 283 · Release-package IG-CUTOVER 强制门禁 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 复用现有 inventory、cutover 和 release-package 作业，不新增检查脚本、transport、worker、registry、parser 或兼容层：

1. `check-generation-cutover-boundary.sh` 现在直接调用 `inventory-generation-paths.sh --check`；退役合同扫描与 machine manifest 漂移由同一个失败即失败入口负责，不能只过符号扫描却漏掉新增 generation 路径。
2. release-package 的 Linux/Windows target matrix 在布局预演前强制运行上述 cutover gate；发布候选若恢复旧合同、出现未分类或 stale inventory key，会在打包前失败。
3. 发布验证暴露 `a4n-core-public-types.tsv` 仍记录已删除的 `event/sse.rs`、`SseEvent` 和 `ProgressTaskKind`。唯一库存快照已改为登记现存 `event/progress.rs` 三个类型，复用已有 `event/` 分类，没有放宽门禁。
4. 本地 Tauri 2.11.2 源码再次确认 MockRuntime 的 `on_window_event` 丢弃回调，只返回 event id；本机也没有 Xvfb/tauri-driver，因此没有新增手工 callback 或 no-op close 来冒充真实 Wry `WindowEvent::Destroyed`。

验证：

```text
generation inventory：actual=46 / declared=46 / missing=0 / stale=0
generation cutover boundary：通过
a4n-core public type inventory：301 declarations；public module inventory：22 modules
sidecar package inventory：8 entries；release manifest generation/tamper：通过
release workflow YAML、release shell/config syntax：通过
deploy dry-run --no-build：linux-x86_64 与 windows-x86_64 均通过
目标 git diff --check：通过
```

`check-refactor-docs.sh` 仍因 `模块化重构执行计划-v2.md` 未被 Git 跟踪而失败；本批不以 `git add` 或放宽脚本掩盖。阶段 10 的 coordinator/artifact/checkpoint/lifecycle 等全量结构门禁、真实 Wry、所有 provider cancel、其他 backend delete、远端 artifact 生命周期和 Windows/macOS 原生 CI 仍缺；工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 284 · Prepared artifact owner 与阶段 10 结构门禁 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 修复真实 owner 缺口，不复制事件或增加运行时分支：

1. `GenerationArtifactCoordinator::available` 不再返回可自由传递的裸 `ArtifactEvent`，改为私有字段的 `PreparedArtifactEvent`；只提供正式 artifact ref 的只读访问和 crate 内消费方法，调用方不能构造或拆出事件。
2. `GenerationTerminalCoordinator` 的 complete/fail/cancel/interrupt 全部只接收 `Vec<PreparedArtifactEvent>`，并在唯一 terminal commit 入口转换为 canonical event。图片、立绘、视频结果和 `GenerationTaskExecution` 同步迁移，类型系统阻止 domain 绕过 coordinator。
3. 唯一 cutover gate 新增阶段 10 已可静态证明的边界：`NullPublisher/collector_only`、领域手写 stream collector、手写 TS generation contract、裸 artifact/checkpoint/terminal/status owner 外引用、owner 外发布、第二 generation cleanup outbox、generation broadcast hub、事件路径/URL 字段和已删除用户删除入口均失败即失败。
4. 共享 `collect_chat_stream` 仍是唯一完整响应 adapter，RAG ingest batch 是 embedding 工作流 owner；二者不因追求扫描数字归零而误删。

验证：

```text
cargo check -p a4n-orchestrator --no-default-features：通过
generation artifact coordinator 精确测试：1/1
terminal artifact-before-terminal 精确测试：1/1
generation inventory：actual=46 / declared=46 / missing=0 / stale=0
扩展后的 generation cutover boundary：通过
全工作树 git diff --check：通过
```

第一次使用宽泛 `artifact` filter 时目标 3 个测试通过，但另命中 6 个 Canvas PostgreSQL 测试；该 shell 未加载 `.env`，6 个均在 runner 因缺 `A4N_TEST_DATABASE_URL` 失败。随后已显式加载现有 `.env` 并运行两个精确 owner 测试，2/2 通过；不能把第一次宽过滤写成整组通过。

阶段 10 的迁移清单逐行关闭证据、持 DB transaction/row guard 等待外部工作静态门禁和全量 IG-VERIFY 仍缺；真实 Wry、所有 provider cancel、其他 backend delete、远端 artifact 和 Windows/macOS 原生 CI 也未关闭。工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 285 · Completed 事务内 permit 等待清零 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 处理阶段 10 的事务等待切片，没有拆坏正式结果与 terminal checkpoint 的原子提交：

1. `GenerationTaskSession::prepare_committed_events` 从异步等待改为同步准备；发现已有 durable commit 时显式返回 session invariant 错误，不再等待 `Notify`。普通 publisher 仍保留事务外的既有异步等待语义。
2. `GenerationTerminalCoordinator` 在取得 checkpoint write guard、开启 completed 事务并锁定 checkpoint 后，不再存在 committed permit await；checkpoint writer 和全部测试调用同步迁移。
3. 全量审计 12 个 `complete_with[_artifacts]` 调用（其中一个为内部转发）：11 个实际 completion participant 只组装 descriptor 或执行明确的 transaction DB 写入，没有 provider、HTTP、文件 IO、sleep、drain 或 semaphore acquire。
4. 唯一 `check-generation-cutover-boundary.sh` 增加同步 API 门禁，禁止把 `prepare_committed_events` 改回 async；没有新增检查脚本、第二 permit owner 或兼容入口。

验证：

```text
cargo check -p a4n-orchestrator --no-default-features：通过
committed permit 冲突/内存/sequence 精确回归：1/1
generation inventory：actual=46 / declared=46 / missing=0 / stale=0
generation cutover boundary：通过
cargo fmt --all -- --check：通过
全工作树 git diff --check：通过
```

第一次精确测试命令额外使用 `--exact` 且未提供完整模块路径，实际运行 0 项，未计为通过；随后去掉 `--exact` 后目标测试真实执行 1/1。测试编译仍有两个既有 chapter-generation support dead-code warning，本批未扩散处理。

本批关闭本地 generation completed/committed permit 的事务等待切片，不宣称已取得 cleanup worker/destructive drain 的完整 `pg_locks` 运行证据，也不关闭阶段 10 迁移清单逐行证据或全量 IG-VERIFY。真实 Wry、所有 provider cancel、其他 backend delete、远端 artifact 和 Windows/macOS 原生 CI 仍缺；工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 286 · 阶段 0/10 迁移清单收敛与逐行裁决 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 清理第二事实来源，不删除仍有职责的 canonical owner：

1. `docs/重构/全量增量生成迁移清单.md` 从过期的 169 个 active machine key/471 个匹配快照重写为当前 46 行固定字段表；每行状态为 `closed-canonical`，证据路径与权威 scanner 一致。
2. `llm_full_response`、`service_task_event`、`legacy_sse_progress`、`frontend_workflow_event` 四类均明确为 0；两个 `manual_stream_collector` 分别保留共享 LLM 完整结果 adapter 和 RAG embedding batch owner；22 个 `media_provider_job` 与 11 个 `structured_stream` 逐行登记唯一 owner。
3. `全量增量生成基线.md` 更新当前静态统计为 `46/175`，同时保留改造前 loopback 测量和真实 UI 延迟/断线恢复/取消 p95 未测事实；不把历史基线或窄扫描冒充阶段 0 整体完成。

验证：

```text
迁移 inventory：actual=46 / declared=46 / missing=0 / stale=0
迁移表固定字段行：46/46
generation cutover boundary：通过
全工作树 git diff --check：通过
```

本批关闭阶段 0/10 的当前入口归类与第二清单漂移，不关闭阶段 0 真实 UI 基线、cleanup/destructive 的完整 `pg_locks` 证据、IG-VERIFY、真实 Wry、provider cancel/其他 delete、远端 artifact 和 Windows/macOS 原生 CI；工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 287 · Provider cleanup capability 逐项裁决 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 复用现有 capability registry 与 cleanup outbox，未为不存在的供应商合同创建 adapter：

1. 当前六种 backend 已逐项裁决：四种异步 backend 支持 status query；`cliproxy-openai-video` 另支持 OpenAI Videos DELETE；同步 `openai-chat-video` 不产生远端 job。当前没有 backend 声明 cancel，Gemini Veo 的官方 v1beta discovery 中 model operation 仅有 `get/list`。
2. NewAPI、Seedance 和 generic compatible adapter 没有仓库可依赖的统一官方 cancel/delete 合同；保持显式 unsupported 是正确能力声明，不能猜测 endpoint 或把通用 DELETE 伪装成 cancel。
3. 非 completed generation terminal 已在同一事务准备独立的 `Cancel` 与 `Reconcile` intent。unsupported Cancel 只以 `outcome=unsupported` 完成自身；Reconcile 仍查询远端状态，pending/running 只增加 observation，completed 原子调度唯一 Delete，cancelled/not_found/failed 显式完成。
4. Delete 仍由唯一 registry 门禁：只有 `delete=true` 才调用 provider adapter；当前仅 OpenAI Videos 路径满足。没有新增 transport、worker、registry、状态机、parser、fallback 或生产代码。

验证：

```text
Gemini 官方 discovery model operations：get/list，无 cancel/delete
a4n-video capability registry 精确测试：1/1
a4n-runtime unavailable operation 不替代精确测试：1/1
a4n-db reconcile observation/deadline 精确测试：2/2
a4n-db completed reconcile 唯一 Delete 精确测试：1/1
generation inventory：actual=46 / declared=46 / missing=0 / stale=0
generation cutover boundary：通过
全工作树 git diff --check：通过
```

一次 `a4n-video` 裸函数名配合 `--exact` 实际运行 0 项，未计为通过，随后去掉 `--exact` 后真实执行 1/1。DB 首次 60 秒冷编译在测试开始时超时，缓存后同命令在 4.2 秒内真实执行 2/2；这次冷编译不计为测试通过。Context7 因月度额度耗尽不可用，外部合同证据改取 Google 官方 discovery。

本批关闭“所有 provider cancel/其他 backend delete 必须实现”的伪关闭条件；未来只有供应商发布稳定官方合同并进入仓库支持范围时才扩展 capability。远端 paid provider 与远端 artifact 生命周期仍未实网验证，阶段 0 真实 UI 基线、cleanup/destructive `pg_locks`、IG-VERIFY、真实 Wry/Destroyed 和 Windows/macOS 原生 CI 也未关闭；工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 288 · Cleanup/destructive 等待期长锁证据 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 只补缺失的运行证据，没有修改生产锁序或新增测试框架：

1. 既有 `concurrent_cleanup_claim_has_one_owner` 在双并发 claim 后新增 `pg_locks` 检查和同一 outbox row 的 `FOR UPDATE NOWAIT`；证明 claim 返回时事务已提交，没有把 lease 期间误实现为数据库长事务。
2. 新增唯一 Server process 用例：启动真实 Server、创建真实 generation reservation，将测试任务置于无法被 drain 强制中断的 `finalizing`，通过生产 `delete_project` API 进入完整 30 秒 destructive wait。
3. project gate 提交为 `deleting` 后、请求仍在 sleep/poll 时，测试检查其他 backend 对 `user`、`novel_project`、`generation_task_checkpoint`、`project_cleanup_outbox` 没有非 `AccessShareLock` 的 granted lock；随后在独立事务中对三行执行 `FOR UPDATE NOWAIT`，并对 outbox 表执行 `LOCK ... NOWAIT`，全部成功。
4. 四类 cleanup claim 都经唯一 `claim_cleanup_by_kind` 短事务；project delete/reset 共用 `drain_project`，user disable 仅把范围扩展为同结构的 `drain_user`。因此证据覆盖共享 owner，而不是为每个调用复制一条 30 秒测试。
5. integration test 文件已有无 feature helper 却错误 gate 其 import；仅把实际无条件使用的 `Arc`、`Body`、`StreamExt/stream` import 移出 feature gate，未改 helper 行为或生产 feature。
6. 最终门禁发现新出现的独立 `crates/qdrant` Git clone 被旧 scanner 当成 A4N 生产路径；唯一 inventory 脚本改为扫描项目约定的 `crates/a4n-*/src`，不硬编码 qdrant 排除、不复制 workspace manifest，也不修改嵌套仓库。

验证：

```text
a4n-db cleanup concurrent claim + pg_locks/NOWAIT：1/1（1.84s）
a4n-server destructive drain PostgreSQL/Server process：1/1（33.51s）
cargo fmt --all -- --check：通过
inventory shell syntax：通过
generation inventory：actual=46 / declared=46 / missing=0 / stale=0
generation cutover boundary：通过
全工作树 git diff --check：通过
```

Server 用例第一次冷编译未产生 runner 结果，不计为通过；无 feature 编译随后先暴露既有错误 gate import，并在修正后进入 runner。首次 runner 又因本地 `.env` 同时设置 `range` 与离散时长变量导致公共视频基线被生产校验拒绝；最终只对测试进程 unset `A4N_PUBLIC_VIDEO_SUPPORTED_DURATIONS_JSON`，没有放宽校验，目标用例真实执行通过。四个既有无 feature 浏览器 helper dead-code warning 保持显性，本批未扩散处理。

本批关闭 cleanup worker claim 与 destructive drain/sleep 的 `pg_locks` 运行证据，不宣称阶段 10 全部完成。阶段 0 真实 UI 基线、阶段 10 其余结构门禁和 IG-VERIFY、远端 provider/artifact 实网验证、真实 Wry/Destroyed 与 Windows/macOS 原生 CI 仍缺；工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 289 · Lifecycle gate entity、幂等迁移与阶段 10 静态门禁 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 修复 schema/entity 单一合同，不在创建入口复制状态机：

1. `user.lifecycle_gate` 和 `novel_project.lifecycle_gate` 进入 canonical SeaORM entity；字段以 `serde(skip)` 保持内部可查询但不进入既有序列化 Host 表面。
2. 新记录继续复用数据库 `DEFAULT 'active' + NOT NULL + CHECK`，所有带 `..Default::default()` 的 ActiveModel 无需机械增加 `Set("active")`；仅 14 个完整 fixture/ActiveModel 构造补齐明确值。
3. 真实 PostgreSQL 首次运行发现 current-entity baseline 已创建列，forward migration 又无条件 `ADD COLUMN`。迁移现用 `ADD COLUMN IF NOT EXISTS`、按 table `regclass` 检查 CHECK 约束和 `CREATE INDEX IF NOT EXISTS`，同时保留旧用户回填与最终 default/not-null 强制。
4. 唯一 cutover gate 新增 entity 映射、Host 禁止使用 project status/user is_active/内存 flag 作 generation gate、compat feature、旧 task-id helper、第二 cleanup worker 和内存补偿队列检查；未增加第二门禁脚本。

验证：

```text
cargo check -p a4n-db -p a4n-orchestrator -p a4n-runtime -p a4n-server --tests：通过（25.35s）
真实 PostgreSQL register/entity + fresh baseline/forward migration：1/1
本批 17 个 Rust 文件定向 rustfmt --check：通过
generation inventory：actual=46 / declared=46 / missing=0 / stale=0
generation cutover boundary：通过
全工作树 git diff --check：通过
```

第一次 PostgreSQL runner 真实失败于 `user.lifecycle_gate already exists`，未记为通过；完成 migration 根因修复后同一测试 1/1 通过。全库 fmt 被本批未触碰的 `a4n-rag-index/a4n-rag-service` 格式差异阻塞；计划状态内容校验通过，但 V2 文档未被 Git 跟踪的既有门禁继续退出 1。七个既有 dead-code warning 未顺手清理。

本批关闭阶段 10 剩余可机械证明的 lifecycle/compat/task-id/cleanup 结构切片，不冒充全量 IG-VERIFY。阶段 0 真实 UI 基线、IG-VERIFY、远端 provider/artifact 实网、真实 Wry/Destroyed 与 Windows/macOS 原生 CI 仍缺；工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 290 · IG-VERIFY 本地固定矩阵与 Host process CI · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 复用现有 runner，删除验证计划中的第二事实来源：

1. 第 23.9 节删除不存在的 `check-generation-event-boundary.sh`、`check-generation-web-e2e.sh`、`check-generation-tauri-e2e.sh` 和会运行 0 项的过滤词，改为当前真实 owner：唯一 cutover gate、Vitest、Server Chromium process、Tauri Channel process、migration 合同与 PostgreSQL generation persistence。
2. 既有 frontend CI 现运行 `npm run test:run`；新增唯一 `generation-host-process` job，使用 PostgreSQL 16、Chrome/ChromeDriver、Node 22 与 Linux Tauri 依赖，顺序执行真实 Chromium 代理断流恢复和 Tauri MockRuntime Channel 重挂载。
3. Chromium 首次运行暴露 `novel_project.lifecycle_gate` 的真实 schema 漂移：current entity baseline 已预建 `NOT NULL` 列，forward migration 的 `ADD COLUMN IF NOT EXISTS ... DEFAULT` 被跳过，导致 ActiveModel 省略该列时没有数据库默认值。migration 现无论列是否预存在都回填 null 并强制 project `DEFAULT 'active' + NOT NULL`；没有在所有项目创建入口复制 `Set("active")`。

验证：

```text
frontend Vitest：6 files / 13 tests
migration 固定合同：3/3
真实 PostgreSQL generation persistence：3/3（runner 8.25s）
真实 PostgreSQL + Server + Vite + Chromium + 断线代理：1/1（runner 14.73s）
真实 PostgreSQL + Tauri MockRuntime Channel：1/1（runner 3.94s）
generation inventory：actual=46 / declared=46 / missing=0 / stale=0
generation cutover boundary：通过
workflow YAML / git diff --check：通过
```

首次 Chromium runner 因 project gate 默认值缺失真实失败，修复 migration 后原路径通过；DB 集成切片第一次只在 60 秒冷编译中超时，缓存后同一命令进入 runner 并通过。另一个会话的 workspace 冷构建持有 artifact lock、被 SIGTERM 后删除共享 `target/`，使第一次排队的 Tauri 命令无诊断退出；空缓存重编后又因本机 `.env` 的离散视频时长污染 range fixture 被生产校验显式拒绝，最终仅在测试进程 unset 该无关变量后 1/1 通过，未放宽校验。

本批关闭本地 IG-VERIFY 核心固定矩阵与 CI 定义缺口，但不宣称发布级 IG-VERIFY 全绿。真实 Wry `WindowEvent::Destroyed`、Windows/macOS 原生 generation CI、远端 paid provider/artifact 生命周期、阶段 0 真实 UI 性能、全 workspace test 与远端 GitHub Actions artifact 仍缺；工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 291 · Web browser harness 首帧与取消 p95 · 2026-07-27

状态：`未完成`

复用批次 290 的唯一真实 Chromium process，没有新增性能框架或依赖。fake OpenAI SSE provider 在每次首帧变为可读时记录 epoch ms；生产 `generationClient`/Zustand reducer 更新后，process harness 把 raw JSON 写入真实 DOM，并在下一 `requestAnimationFrame` 记录首帧。连续 5 个独立 task 取中位数。取消路径另运行 20 个独立 task，每次先收到 delta 再调用生产 cancel API，以唯一 cancelled 终态作为停止发布的保守上界并计算 p95。

验证：frontend Vitest 13/13、frontend production build、Rust 定向 rustfmt 与 diff check 通过；真实 PostgreSQL + Server + Vite + Chromium process 1/1（runner 27.22s）。首帧样本 `[105, 105, 105, 105, 587]ms`，中位数 `105ms`，通过 Web `250ms` 门禁，首轮 warm-up 的 587ms 未删除；取消样本范围 `31～43ms`，p95 `38ms`，通过 `500ms` 门禁。

这关闭的是 production-client browser harness 的 provider→DOM 首帧中位数和 Web 取消 p95 切片，不冒充完整产品 React 页面或 Tauri/Wry UI。阶段 0 仍缺 Tauri 首字、产品 React 可见提交计数和 10,000 chunk UI batching/容量；远端 provider/artifact、真实 Wry/Destroyed、跨平台 CI、全 workspace 与远端 Actions 也未关闭。工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 292 · 10,000 chunk 发布与 React 可见提交容量 · 2026-07-27

状态：`未完成`

继续复用唯一 Chromium process。fake OpenAI provider 仅对容量请求发送恰好 10,000 个按 `1..8` 字节循环的正文 chunk；生产统一 stream driver 按既有 50ms/16KiB 规则 batching，Web 侧经真实 SSE、`generationClient`、Zustand store、`useGenerationTask` 和 React Profiler 写入 DOM。没有改生产 owner，也没有增加依赖、第二 transport、状态源或性能框架。

真实结果为 `214` 个发布事件、`196` 次 React 可见提交，分别通过第 19.3 节的 `<=1,000`/`<=300` 门禁；最终 DOM 和 authoritative state 都是 45,000 字节，SHA-256 均为 `7adbe9eb84b731c3dfb24e58bc2b49502345d2df701f1472906d751d8efe7e20`。同一 runner 1/1 通过（44.69s），最新首帧 `[108, 108, 108, 109, 483]ms`、中位数 108ms，取消 `46～90ms`、p95 79ms，原断流/cursor/checkpoint/artifact/唯一终态断言仍全部通过。frontend production build、Server 无 feature 与 LLM/media feature 两种 test check 也通过；首次无 feature check 暴露 `SystemTime` 条件导入过窄，修正后原命令通过。

这关闭 10,000 chunk 的发布事件、真实 React 消费链可见提交和最终 hash 切片；该 React 探针使用生产 hook/renderer，但不冒充完整产品页面 E2E。阶段 0 仍缺 Tauri/Wry 首字、完整产品页和慢消费者恢复等证据；远端 provider/artifact、真实 Wry/Destroyed、跨平台 CI、全 workspace 与远端 Actions 也未关闭。工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 293 · 慢消费者 replay floor 与 snapshot 无损恢复 · 2026-07-27

状态：`未完成`

没有新建恢复机制，只补现有唯一合同的缺失断言：Orchestrator session 在最小合法 `replay_max_events=256` 下发布 256 个正文事件，旧 cursor 明确得到 `SnapshotRequired`；snapshot 覆盖 sequence 257 且含完整 256 字节正文。模拟下一次持久 checkpoint 后再发布 `tail`，按 snapshot covers replay 只得到 sequence 258、offset 256 的后续 delta，拼接结果无缺口。

frontend 现有 production Web transport 测试新增同一流程：首次 SSE 返回规范 409 `generation_snapshot_required`，client 拉取 covers=257 的 snapshot，再用 `Last-Event-ID: 1:257` 重挂载并接收 `tail`/completed；最终 store 正文完整、next sequence=260、recovery 回到 idle。Vitest 2/2、Orchestrator 定向 runner 1/1 和两文件 rustfmt 通过；Rust 首次 60 秒只冷编译未进 runner，缓存后首次 runner 又因测试忽略 sequence lease 已提前把 checkpoint revision 推进到 2 而失败，改为模拟下一 revision=3 后原断言通过，生产 revision 校验未放宽。

该组合证据关闭“慢消费者触发 snapshot 恢复时不静默缺字”的本地合同切片，但不冒充一次真实网络限速 process。阶段 0 仍缺 Tauri/Wry 首字、完整产品页和其余容量证据；远端 provider/artifact、真实 Wry/Destroyed、跨平台 CI、全 workspace 与远端 Actions也未关闭。工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 294 · 1 MiB 正文与 10 次断线重连固定证据 · 2026-07-27

状态：`未完成`

复用既有 `server_process_reconnects_inspiration_sse_ten_times_without_losing_one_mib`，不写第二测试。真实 PostgreSQL/Server/fake provider 生成 1 MiB 正文；客户端连续 10 次每次读取一个非终态事件后主动断开，再按最后 cursor 重连，最终逐项断言 task/attempt/sequence、usage、唯一且最后的 completed、stream/terminal/summary/database 正式结果一致。

本地 runner 1/1 通过（10.16s，含编译总命令 29.31s）：最终正文 1,048,576 字节，SHA-256 `8f990ba0b577b51cf009ea049368c16bbda1b21e1b93be07a824758bb253c39b`，规范事件总数 76。现有 `generation-host-process` CI 在 Chromium 后新增同 feature 的该 runner，复用编译缓存与 PostgreSQL service；全量生成计划第 23.9 节同步登记唯一命令。

这关闭第 19.3 节 1 MiB hash、terminal 前最后 batch 可见和“同一任务 10 次断线”切片；100 并发订阅、多任务交错、session 内存/TTL、Tauri/Wry 和完整产品页仍未关闭。远端 provider/artifact、真实 Destroyed、跨平台 CI、全 workspace 与远端 Actions也仍缺；工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 295 · 100 并发订阅、多任务隔离与内存回落 · 2026-07-27

状态：`未完成`

继续使用唯一 `GenerationAdmissionController` 和 `GenerationTaskSession`：两个不同 task 各保留 50 个订阅，在同一循环交错发布各 100 个 `a`/`b` delta。每个订阅者从 cursor=1 拉取 replay，必须得到本 task 的 100 个事件和完整正文，任何 task id 或内容串流都会失败。测试结束显式释放 subscriptions、publishers 和 sessions，并断言 active sessions、session memory bytes、subscriptions 全部为 0。

现有 terminal registry 测试同步增加 TTL 清理后的同一三项精确归零断言；既有 65th active session 和 1,025th Host subscription 容量拒绝仍保留。`cargo test -p a4n-orchestrator generation::events::tests:: --lib` 进入 runner 后 14/14 通过（runner 0.06s，总命令 23.88s），仅有两个既有 chapter test-support dead-code warning。

本批关闭第 19.3 节 100 并发订阅、多任务交错不串流和 session/terminal TTL 内存回落的本地切片。它不冒充真实 Tauri/Wry 或完整产品页面；远端 provider/artifact、真实 Destroyed、跨平台 CI、全 workspace 与远端 Actions也仍缺，工作包 09、3.15/3.16、阶段 F/G 和总体 V2 保持 `未完成` / `16/44`。

## 批次 296 · Completed hash 校验与 snapshot 二次确认 · 2026-07-27

状态：`未完成`

本批按 `ponytail full` 只修改唯一 generation client：completed 进入 reducer 前，使用既有 Web Crypto `sha256Hex` 校验全部可见 stream 的数量、唯一 id、UTF-8 字节数和 SHA-256。首次不一致复用现有 snapshot recovery；terminal snapshot 的正文、offset、snapshot hash 与 completed hash 全部一致后才发布成功，二次不一致则保持 `terminalReceived=false`、进入 `consistency_error` 并让 `waitForTerminal` 明确失败。Web SSE 与 Tauri Channel 共用同一入口，没有新增状态机、transport、parser、依赖或正式结果 repository；`sha256Hex` 仅从浏览器专用 `window.crypto` 改为平台标准 `globalThis.crypto`。

验证：frontend 定向 5/5、完整 Vitest 6 files/17 tests、生产 `tsc + Vite build` 通过。测试覆盖本地 hash 直接匹配、首次不匹配后 terminal snapshot 校正、snapshot 仍不匹配时不得成功三条路径。

本批关闭公共 client 的 completed final hash 静默成功缺口，但不勾选“hash/revision + 正式结果只读展示和显性告警”完整关闭条件：领域 `result_ref.revision` 查询仍由既有页面 owner 负责，完整产品页只读告警证据尚缺。Tauri/Wry、远端 provider/artifact、全 workspace、远端 Actions 和跨平台 CI 也仍缺；工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 297 · Canonical result_ref 与正式结果只读错误链 · 2026-07-27

状态：`未完成`

批次 296 暴露的相邻缺口已从唯一状态边界修复：hash 二次失败不能 dispatch completed，因此原先也丢失了 `result_ref`，页面无法按正式 owner 恢复。本批新增一个 reducer `consistency_error` action，原子保留 canonical `result_ref`、设置 client/recovery error，同时继续保持 `terminalReceived=false`；没有增加 pending terminal 缓存、异常层或第二状态源。

四个真实文本消费者现同时处理 verified terminal 与 transport consistency error。章节读取正式章节并锁为当前章节只读核对态；灵感消息显示正式消息正文和持久警告；蓝图显示正式蓝图但隐藏优化/确认动作；Optimize 先刷新正式 session/turn，再追加显性校验错误。revision 本身仍由各领域既有查询 owner 校验，公共 client 不包含领域 repository。Tauri Channel 使用同一 handler 的错误 snapshot 用例已通过。

验证：generation client/reducer/只读组件定向 3 files/14 tests，完整 frontend 7 files/21 tests，生产 `tsc + Vite build` 通过。第一次 build 因新增 SSR 测试的未使用 React import 被严格类型检查拒绝，删除该无效 import 后原命令通过。

本批完成“hash 二次失败后按 result_ref 加载正式结果、只读展示并显性告警”的本地代码和组件证据，但完整产品页 Chromium/Tauri UI process 尚未执行，因此第 22.4 节发布关闭项仍不勾选。真实 Wry/Destroyed、远端 provider/artifact、全 workspace、远端 Actions 和跨平台 CI 也仍缺；工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 298 · 真实灵感产品页 Chromium generation · 2026-07-27

状态：`未完成`

继续复用唯一 `real_chromium_recovers_generation_after_proxy_aborts_sse`。现有断流代理只保留 generation events 的首次 connection reset，并以同一个 Axum router fallback 透明转发真实产品页所需的 auth/project/history/command 请求；没有新增代理、runner、浏览器依赖或生产 transport。fake provider 的定向响应使用灵感页面既有完整 JSON 合同，不绕过 production parser。

同一 ChromeDriver session 完成 process harness 后导航到真实 `/inspiration/:id`，由 `AuthGate` 读取同源 portable storage 登录态。测试通过 WebDriver 找到页面 textarea 和“发送”按钮，输入唯一消息并点击；最终同时断言实际 App DOM 出现项目页、用户消息和正式 assistant 正文，PostgreSQL `message` 保存完整正式响应，`generation_task_checkpoint` 为 completed 且 `result_kind/result_id/target_kind` 指向该正式消息与灵感会话。

真实 PostgreSQL + Server + Vite + Chromium runner 最终 1/1 通过（44.82s）；同轮容量发布 210、React commits 189、Web 首帧中位数 107ms、取消 p95 87ms，旧断流/cursor/hash/artifact/terminal 断言仍全部执行。前两次新断言分别暴露 fake 路由被历史容量消息抢先匹配、简化 `reply` JSON 不满足真实页面 parser，均在测试 fixture 根因处修正后复验；未改生产代码。

本批关闭 Web 完整灵感产品页“正常 generation → DOM → 正式 message/task”process 切片，不冒充 hash 二次失败的产品页错误注入、Tauri/Wry UI 或第 22.4 节发布级完成。真实 Wry/Destroyed、远端 provider/artifact、全 workspace、远端 Actions 和跨平台 CI 仍缺；工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 299 · 真实产品页 completed/snapshot 一致性错误链 · 2026-07-27

状态：`未完成`

继续使用批次 298 的唯一 Chromium runner、真实 `/inspiration/:id` 和同一个 Axum 代理。第一个产品任务保留正常成功；随后测试启用一个共享原子计数开关，代理只对第二个任务以 `serde_json` 结构修改 SSE completed 和 snapshot terminal 的首个 SHA-256。计数必须从启用值 1 增至至少 3，证明 completed 与 snapshot 都经过破坏；生产 Server、checkpoint、正式 message 和 result columns 保持 canonical，不增加第二 transport/parser/状态机。

实际页面在第二次输入/点击后必须同时显示唯一 user 消息、从 `result_ref` 重新读取的正式 assistant 正文和持久告警“生成结果一致性校验失败，以下内容来自正式消息，只供核对”。PostgreSQL 再核对相邻 user/assistant、completed task、`result_kind/result_id/target_kind/target_id` 及正式正文 SHA-256 revision。第一个正常任务还必须等待“发送中”消失并确认没有 consistency warning，防止把 streaming draft 当成最终成功。

本批审计发现批次 298 的 fake `ui_control: {}` 不符合真实 parser，旧 DOM 条件在流式草稿出现时提前通过。前三次扩展 runner 分别停在无告警、确认 wire 双重破坏但无正文、最终 DOM 显示 `❌ ui_control.type 无效`；fixture 改为现有 `text_input` 合同且正常等待条件收紧后，最终 runner 1/1（48.34s）。同轮发布事件 211、React commits 195、首帧中位数 108ms、取消 p95 83ms；inventory `46/46`、cutover、rustfmt 和 diff check 通过。

本批纠正并取代批次 298 的弱 DOM 证据，同时关闭灵感产品页的真实 `completed mismatch → snapshot mismatch → consistency_error → formal result warning` process 切片。其他文本领域完整页面、expired/corrupt summary 分支、Tauri/Wry、真实 Wry/Destroyed、远端 provider/artifact、全 workspace、远端 Actions 和跨平台 CI 仍缺；第 22.4 节发布级条目、工作包 09、3.15/3.16、阶段 F/G 与总体 V2 继续 `未完成` / `16/44`。

## 批次 300 · Expired/corrupt 四分支与真实产品页恢复 · 2026-07-27

状态：`未完成`

唯一 reducer 旁新增纯 `generationTaskRecoveryIssue` 判定，将 `consistency_error`、`expired`、`corrupt` 收敛为页面终结原因；没有新状态、transport、parser、repository 或 fallback。章节、灵感消息、蓝图和 Optimize 四个现有 consumer 都在 expired/corrupt 后结束 busy：有 canonical `result_ref` 时继续调用各自既有领域查询并只读展示正式对象及原因告警；无引用时不猜结果，显示明确不可恢复错误并清理 generation handle。查询/校验异常不会再把 expired/corrupt 覆盖成第二个 consistency 状态。

production Web transport 以一个表驱动用例覆盖 expired/corrupt × 有/无 `result_ref`：events 409 → snapshot 失败 → summary → detached，每个分支固定三次请求且不重连。两份 frontend 测试 20/20、完整 TypeScript `tsc --noEmit` 通过。

唯一 Chromium runner 的现有 Axum 代理扩展为故障模式，不新增入口或浏览器 session。代理先等待真实任务完成，再依次返回 events 409、snapshot 409，并只在 wire summary 上修改 `snapshot_state`/可选清空 `result_ref`；四个分支均在真实 `/inspiration/:id` 输入并点击发送。有引用分支显示正式 assistant 正文和对应过期/损坏告警，无引用分支显示“且没有正式结果引用，无法恢复”，发送态全部结束；PostgreSQL 六次产品任务仍逐项保持正式 user/assistant、completed checkpoint、canonical result/target columns 与 revision。

最终真实 PostgreSQL + Server + Vite + Chromium 1/1（65.56s）；四个恢复分支各命中 events/snapshot/summary 至少三次。同轮容量发布 214、React commits 193、首帧中位数 108ms、取消 p95 89ms。全库 fmt 仍被本批未触碰的 `a4n-rag-index/a4n-rag-service` 既有格式差异阻塞，不冒充通过。

本批关闭 expired/corrupt summary 的 Web transport 和灵感完整产品页四分支，不代表章节、蓝图、Optimize 完整页面或 Tauri/Wry 已实跑。其他文本领域产品页、真实 Wry/Destroyed、远端 paid provider/artifact、全 workspace、远端 Actions 和 Windows/macOS 原生 CI 仍缺；第 22.4 节发布级条目、工作包 09、3.15/3.16、阶段 F/G 与总体 V2 继续 `未完成` / `16/44`。

## 批次 301 · 四个文本领域真实产品页与正式结果恢复 · 2026-07-27

状态：`未完成`

继续复用唯一 `real_chromium_recovers_generation_after_proxy_aborts_sse`、同一 PostgreSQL/Server/Vite/ChromeDriver session、既有页面 controller 和领域 repository。灵感正常、completed/snapshot 双 hash mismatch、expired/corrupt × 有/无 `result_ref` 六分支继续通过；同一浏览器随后顺序实跑蓝图、章节正文、Optimize 的正常生成，以及三者各自 expired + `result_ref` 的只读恢复。没有新增 runner、生产 transport、状态源、parser、repository、依赖或 fallback。

蓝图正常页验证四段 staged provider JSON、正式 blueprint version 1.0 和 `blueprint_generation → novel_blueprint` result/target 合同；恢复页按正式 version 1.1 只读展示。章节页验证正式正文、`status=draft`、completed checkpoint、`result_kind=chapter`、`target_kind=chapter` 与正文 SHA-256 revision；恢复页读取同一正式章节。Optimize 页验证 completed Turn、相邻正式 user/assistant message、`result_kind=optimization_turn`、`target_kind=optimization_chat` 与 reply SHA-256；恢复页读取下一正式 Turn。项目标题被蓝图正式结果更新后，后续等待使用 canonical 新标题或稳定页面标记。

WebDriver 辅助器只做必要稳定化：从全部匹配项选择可见且启用的按钮/textarea、仅对明确 click intercepted 等待 toast、将确认按钮限定在 dialog、蓝图后真实点击“查看蓝图”关闭对话框，并在第二次蓝图生成前通过既有路由重挂载页面。Chrome 的 `--window-size=1440,1000` 只固定桌面 runner viewport，避免 Optimize composer 被小视口裁切，不改变生产布局或断言合同。

最终真实 PostgreSQL + Server + Vite + Chromium 1/1（53.17s）；发布事件 210、React commits 192、首帧样本 `[106, 107, 108, 108, 535]ms`、中位数 108ms，取消样本 29～78ms、p95 77ms。目标测试编译和文件级 rustfmt 均通过，原容量、首帧、取消、断流/cursor/hash/artifact/terminal 断言继续执行。

本批关闭 Web 文本领域灵感、蓝图、章节、Optimize 的真实完整产品页 process 切片，不代表真实 Tauri/Wry provider→UI 首字、Tauri 窗口重挂载或 `WindowEvent::Destroyed` 已验证。远端 paid provider/artifact 生命周期、全 workspace test、远端 GitHub Actions artifact、Windows/macOS 原生 CI 和发布级完整 IG-VERIFY 仍缺；工作包 09、3.15/3.16、阶段 F/G 与总体 V2 继续 `未完成` / `16/44`。

## 批次 302 · 真实 Tauri/Wry 重挂载、首帧与 Destroyed 清理 · 2026-07-28

状态：`未完成`

继续复用唯一 `real_chromium_recovers_generation_after_proxy_aborts_sse` 综合 runner 和既有 Wry E2E；没有新增页面、transport、综合 Rust runner 或测试框架。Wry 通过真实 `.chat-composer textarea` 发起 generation，活动任务按 cursor 重挂载后回复只出现一次；测试再调用 Tauri 官方 `plugin:window|destroy`，生产 `WindowEvent::Destroyed` 路径记录 `window_label="main" subscriptions_cancelled=1`，窗口销毁时任务仍可继续发送，已销毁窗口不再持有订阅。

桌面可执行文件必须由 `cargo build -p a4n-tauri --features embedded-capabilities` 构建；`default = []` 是检查型瘦构建，不包含 embedded LLM provider。capability 只增加既有官方权限 `core:window:allow-destroy`。provider connect 失败继续返回稳定公共错误，同时在生产边界记录底层错误；Optimize 产品页断言改为轮询正式 Turn 到 `completed`，避免把流式 DOM 出现误当成数据库事务已经提交。共享视频 `frame_segment_plan` 的写入和复用查询统一经 `PromptWorkflowMode::storage_str()` 规范为数据库 `shared_segment_plan`，没有增加第五种工作流模式或第二状态源。

最终真实 PostgreSQL/Server/Vite/Chromium/Tauri/Wry runner 1/1（92.22s）。Tauri provider→UI 首帧样本 `[131, 133]ms`、中位数 132ms，通过 175ms 门禁；Web 样本 `[105, 106, 106, 107, 482]ms`、中位数 106ms，通过 250ms 门禁；取消 20 次样本 49～68ms、p95 64ms，通过 500ms 门禁。10,000 个上游 chunk 最终为 223 个发布事件、201 次 React 可见提交、45,000 字节，SHA-256 `7adbe9eb84b731c3dfb24e58bc2b49502345d2df701f1472906d751d8efe7e20`；灵感六分支及蓝图、章节、Optimize 正常/恢复链继续通过。并行新增的视频提示词 application 已归入权威清单，inventory/cutover 为 `47/47`。

本批关闭本地 Linux 真实 Tauri/Wry 首帧、活动 cursor 重挂载、回复唯一性和 Destroyed 订阅清理切片，不冒充远端 paid provider、远端 artifact 实网生命周期、远端 GitHub Actions artifact、Windows/macOS 原生 CI、全 workspace test 或发布级完整 IG-VERIFY。Orchestrator 共享规划目标单测两次均在 60 秒内只完成聚合二进制编译/链接，未进入用例，不计为通过；综合 runner 已真实覆盖当前 Orchestrator/Server 链路。工作包 09、3.15/3.16、阶段 F/G 与总体 V2 保持 `未完成` / `16/44`。

## 批次 303 · 工作包 09 关闭条件最终裁决 · 2026-07-28

状态：`完成`

本批没有继续增加代码或重复长 runner，而是逐条按 V2 3.10、3.15、3.16 与阶段 A/F/G 的原始关闭条件审计当前源码和批次 226–302 的运行证据。生命周期 33 个入口已有 owner/DB owner/补偿/恢复/TTL 矩阵；持久创建只落在六个受管媒体根或尚未提交的新项目目录，启动 scanner、canonical DB 引用复核和唯一 cleanup outbox 可跨重启恢复，DB/文件双向失败、强杀、claim/lease/retry 已由 PostgreSQL/Server process 覆盖。四个直接物理删除点仅处理临时上传、partial output 或失败目录回滚。

事件边界只剩 canonical `GenerationTaskEvent`。Server SSE 与 Tauri Channel 复用同一 session/replay/snapshot；真实 Chromium 与真实 Linux Wry 已覆盖 cursor、usage、checkpoint、artifact、cancel ack、唯一终态、重挂载和 `WindowEvent::Destroyed` 清理。旧 `ServiceTaskEvent`/`HostTaskEvent`/`SseEvent`/通用 channel 和无消费者 fallback 均由 cutover gate 禁止回流。

阶段 A 复核结果为 runtime 本地纯 rules `0 文件 / 0 行`、Orchestrator 透传壳 `0/0`、actions 生产 `.orchestrator()`/`state.db` 直连 `0/0`。因此 3.10、3.15、3.16 与阶段 A/F/G 同时关闭，工作包 09 完成；总体从 `16/44` 更新为 `22/44`。远端 paid provider/artifact、GitHub Actions、Windows/macOS、发布签名/供应链属于 3.17、3.19–3.26 和工作包 10，不再作为本工作包的虚假代码缺口。
