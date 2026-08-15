# 参考媒体视频 Provider 请求预览单一合同清理记录

日期：2026-08-14

## 结论

`preview_reference_media_video_request` 是章节“片段生成”中的活跃生产功能，不能整体删除。BFS 确认冗余集中在预览专用第二合同：前端允许跳过媒体、URL、用途、Provider 和时长校验；后端又把过短 Prompt、非法 URL、缺失 token、超量媒体、不支持的 Provider 以及空媒体请求改写成占位值和 note，导致预览展示的是生成路径不会接受的伪请求。

本批将预览和生成收敛到同一严格 preparation 与 Provider 请求构建合同。任何生成无效输入现在在预览时同样显式失败。合法本地文件仍可只读预览，但必须先通过文件名、MIME、SHA-256、base64 内容、保存槽位和用途校验，随后仅展示绑定该 SHA-256 的脱敏 URL；该表示不接受缺失或无效输入，也不会进入真实生成请求。

未新增兜底、兼容、回退、无效输入占位、适配器或第二套校验。

## BFS 调用拓扑

```text
章节视频“查看模型请求”
  -> ReferenceMediaWorkspace.onPreview
  -> VideoTab.prepareReferenceMediaVideoInput
  -> previewReferenceMediaVideoRequest
  -> preview_reference_media_video_request Host command
  -> Server command dispatch / Tauri generic dispatch
  -> Runtime preview_reference_media_video_request
  -> Orchestrator preview_reference_media_video_request_validated
  -> prepare_reference_media_video_generation(Preview)
  -> reference_media_rules 唯一严格合同
  -> RuntimeVideoProviderHostPort
  -> sidecar 或 embedded provider 正式 request body builder

真实生成兄弟路径
  -> start_video_generation(reference_media)
  -> generation reservation 输入校验
  -> run_reference_media_video_generation_validated
  -> 同一个 prepare_reference_media_video_generation(Generate)
  -> 同一个 reference_media_rules 严格合同
  -> 校验与容量检查通过后暂存本地媒体
  -> 正式 Provider 请求、记录、checkpoint、提交与结果持久化
```

## BFS 搜索覆盖范围

### 第 0 层：用户入口与表单状态

- `frontend/src/pages/novel-detail/VideoTab.tsx`
- `frontend/src/pages/novel-detail/video/referenceMediaWorkspace.tsx`
- `frontend/src/pages/novel-detail/video/referenceMediaWorkspace.test.ts`

确认“查看模型请求”和“生成视频”读取同一个片段草稿、共享规划身份和本地上传对象。原 `ReferenceMediaInputPurpose` 与 `collectReferenceMediaRows(..., "preview")` 构成明确的预览绕过。

### 第 1 层：前端 API 与生成合同

- `frontend/src/api/video/records.ts`
- `frontend/src/api/video/types.ts`
- `frontend/src/api/generated/generationContracts.ts`
- `frontend/src/api/generated/host.ts`

预览与生成均使用 `ReferenceMediaVideoInput`；Prompt、URL、用途、时长和画幅来自已保存的当前片段 recipe，调用参数只携带稳定片段目标、本地上传和可选配置 ID。

### 第 2 层：命令清单与宿主分发

- `crates/a4n-api-contract/src/video.rs`
- `crates/a4n-api-contract/src/command_manifest.rs`
- `crates/a4n-server/src/routes/commands/dispatch/video.rs`
- `crates/a4n-server/src/routes/commands/media/video_generation/input.rs`
- `crates/a4n-server/src/routes/commands/media/video_generation/preview.rs`
- `crates/a4n-tauri/src/command_registry.rs`
- `crates/a4n-tauri/src/commands/dispatch.rs`

命令清单将预览标记为只读、`without_secrets`。Server 显式解析参数，Tauri 通过统一注册表进入同一 Runtime action；未发现桌面或 Web 专用隐藏实现。

### 第 3 层：Runtime action 与 Host Port 注入

- `crates/a4n-runtime/src/actions/video_generation/reference_media/api.rs`
- `crates/a4n-runtime/src/actions/video_generation/provider_host.rs`
- `crates/a4n-runtime/src/actions/video_generation/contracts.rs`
- `crates/a4n-runtime/src/sidecar_clients/video/operations/sidecar.rs`

`RuntimeVideoProviderHostPort` 是唯一生产实现，同时负责配置解析、Provider 请求体、真实提交和远端临时媒体上传。预览不会调用上传方法；生成现在直接接收 Orchestrator 已解码并校验的字节，不再重复解析 base64。

### 第 4 层：Orchestrator 编排、recipe 与执行身份

- `crates/a4n-orchestrator/src/video/reference_media_generation.rs`
- `crates/a4n-orchestrator/src/video/reference_media_rules.rs`
- `crates/a4n-orchestrator/src/video/generation.rs`
- `crates/a4n-orchestrator/src/video/prompt_application.rs`
- `crates/a4n-core/src/video_execution/segment_recipe.rs`

覆盖项目编辑权限、章节来源、当前共享片段规划、稳定片段目标、recipe workflow、执行身份、配置、时长、画幅、Prompt、媒体槽和 Provider 能力。预览与生成只在“合法本地媒体如何表示/传输”处保留必要差异。

### 第 5 层：本地上传、远端暂存与清理 artifact

- `crates/a4n-runtime/src/actions/video_generation/media/provider_images/remote_upload.rs`
- `crates/a4n-runtime/src/actions/video_generation/media/reference_images/transport.rs`
- `crates/a4n-service-protocol/src/`

生成路径会将已校验字节上传为有用户、项目、来源媒体、SHA-256、大小和过期时间身份的临时 artifact。请求合同和并发容量检查现在先于暂存，避免已知无效请求或已满容量先产生远端临时文件。

### 第 6 层：Provider 正式请求体

- `crates/a4n-video/src/lib.rs`
- `crates/a4n-video-providers/src/factory.rs`
- `crates/a4n-video-providers/src/seedance2/request.rs`
- `crates/a4n-video-providers/src/cliproxy_openai/request.rs`
- `crates/a4n-video-service/src/providers.rs`

Seedance 2.0 使用 `functionMode=omni_reference` 与图片、视频、音频列表；CLIProxy/Grok 只接受图片参考并限制数量。sidecar 与 embedded profile 都调用同一正式 builder，没有预览专用 Provider 请求实现。

### 第 7 层：测试与隐式依赖排查

- `crates/a4n-orchestrator/src/video/tests/provider_generation_application.rs`
- Runtime profile 与 `embedded-media-providers` feature
- 仓库 `.env` 与 `docs/TEST_DATABASE.md`

仓库未发现本功能相关的反射调用、动态代理、AOP、XML、注解路由或 DI 容器隐藏绑定。Rust Trait、命令清单、feature 和 Provider 工厂均为静态显式依赖；数据库专项按文档加载隔离测试库并串行运行。

## 入向引用闭包

- `REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER` 只服务过短 Prompt 软化。
- `reference_media_preview_placeholder_url` 同时服务非法 URL、空媒体和本地上传三类预览占位，混淆了无效输入与合法脱敏输入。
- `validation_error_or_preview_note` 只把生成错误转换成预览 note。
- `ensure_reference_media_tokens` 的预览分支会自动追加用途；严格分支在当前 recipe Prompt 构造后不可触达，因为每个已校验行都会统一生成对应 token 绑定行。
- `provider_input_note` 及两个 append helper 只承载预览软化结果。
- 前端 `ReferenceMediaInputPurpose`、`collectReferenceMediaRows` 和 `purpose === "preview"` 只构成第二套宽松校验。
- `reference_media_upload_slot_count` 在上传身份校验后重复计算相同槽位数量。
- Runtime 上传实现重复解码 Orchestrator 已为 SHA 校验解码过的同一 base64。

## 优化记录

1. 删除前端 `ReferenceMediaInputPurpose` 双模式类型。
2. 删除 `collectReferenceMediaRows` 第二套行校验实现。
3. 删除预览直接返回未校验活动行的绕过。
4. 预览与生成统一调用 `referenceMediaDraftError`。
5. 将原 `referenceMediaGenerateDraftError` 改名为不带生成偏向的共享合同名。
6. 新增前端唯一 preparation 流程，统一片段解析、草稿校验、保存和输入构建。
7. 预览改为执行与生成相同的 Provider 配置和时长 gate。
8. 无效草稿时禁用“查看模型请求”，不再发起必败请求。
9. 删除前端仅供旧分叉使用的常量、类型和 URL helper 公共导出。
10. 后端预览进入 preparation 时复用 `validate_reference_media_start_input`。
11. 删除 preparation 重复传入的 operation 与 writable 参数，由模式唯一推导。
12. 删除通用预览 Prompt 占位常量。
13. 删除通用预览 URL 占位函数。
14. 删除生成错误转预览 note 的通用软化 helper。
15. 删除过短 Prompt 自动拼接占位文本的路径。
16. 删除非法或空 URL 自动替换为 `example.invalid` 的路径。
17. 删除缺失 token 自动追加用途占位说明的路径。
18. 删除当前 recipe Prompt 构造后不可达的 token 二次扫描。
19. 不支持的 Provider 在预览和生成中统一失败。
20. 不支持的时长在预览和生成中统一失败。
21. 超过 Provider 媒体数量上限时统一失败。
22. 删除空媒体请求伪造首张图片的路径。
23. 删除 CLIProxy/Grok 缺图时伪造图片的路径。
24. 删除 CLIProxy/Grok 超量图片静默截断。
25. 删除 CLIProxy/Grok 视频/音频限制的两次分叉检查，保留一个严格错误。
26. 删除 Provider note 中“不支持但仅预览结构”的回退说明。
27. 删除 `PreparedReferenceMediaVideoRequest` 包装结构。
28. 删除 `provider_input_note` 字段及整条透传链。
29. 删除两个 note 追加 helper。
30. 删除上传身份校验后的重复槽位数量检查。
31. 将媒体数量上限收敛到 Provider 请求构建层唯一校验。
32. 将上传身份校验改为返回已验证的文件名、MIME、字节和 SHA-256。
33. Runtime Host Port 直接接收已校验字节，删除第二次 base64 解码。
34. 先完成 Prompt、URL、媒体数量、Provider、时长和画幅预检，再进行远端暂存。
35. 将并发容量检查前移到远端暂存之前。
36. 合法本地媒体预览改为使用绑定实际 SHA-256 的脱敏 URL，不再与无效输入占位共用函数。
37. 脱敏 URL 仅存在于 Preview request；Generate 在提交前替换为真实暂存 URL。
38. 增加“生成无效 recipe 的预览必须失败且不得调用 Provider builder”回归测试。
39. 增加“合法本地上传只返回 SHA 绑定脱敏表示、不泄露 base64、也不提交 Provider”回归测试。
40. 修正过期的 Seedance 测试配置时长范围 `1..60` 为当前能力 `4..15`。
41. 修正过期的共享片段规划测试正文哈希，改为从真实章节内容计算。

## 规模

- 修改 8 个既有 Rust/TypeScript 源码或测试文件，新增 1 个清理记录文件。
- 源码与测试净变化 `+372/-579`，净删除 207 行。
- `reference_media_rules.rs` 从 841 行降至约 600 行，删除预览软化主干。

## 验证记录

- `cargo fmt --all`：通过。
- 读取 `docs/TEST_DATABASE.md` 并从仓库 `.env` 加载 `A4N_TEST_DATABASE_URL` 后串行执行参考媒体 Rust 专项：原批 18/18 通过；新增本地上传脱敏专项 1/1 通过。
- 首轮 Rust 专项暴露测试夹具仍使用旧 Seedance 时长范围和固定正文哈希；确认生产合同后修正夹具，未修改生产逻辑绕过。
- `npm run test:run -- referenceMediaWorkspace.test.ts`：2/2 通过。
- `timeout 60s cargo check -p a4n-orchestrator -p a4n-runtime --all-targets`：通过；Cargo 仅报告全局缓存 last-use 数据写入的磁盘 I/O 环境警告，退出码为 0，两个 crate 均完成检查。
- `timeout 60s npm run build`：TypeScript 检查与 Vite 生产构建通过。
- `git diff --check`：通过。
- 退役 Prompt/URL 占位、软化 helper、token 自动补齐、前端 purpose 分叉和旧函数名在 `crates/`、`frontend/` 中 `rg` 零命中。
- 所有 Cargo 命令直接复用仓库 `/home/fufu/Code/A4N/target/`；未在 `/tmp` 修改源码或建立替代 target。

## 保留边界

- 保留活跃的参考媒体 Provider 请求预览入口及请求弹窗。
- 保留项目编辑权限、章节来源、当前共享规划、稳定片段身份和 source hash 校验。
- 保留 recipe 草稿可暂存不完整输入的能力；只有预览和生成要求完整严格合同。
- 保留合法本地媒体的脱敏请求表示，因为只读预览不得返回原始 base64、签名 URL 或创建远端临时 artifact。
- 保留 Preview/Generate 模式枚举，因为章节视频和自由画布仍使用它区分只读媒体表示与真实传输；本批只删除参考媒体规则中的软化语义。
- 保留生成路径的远端临时媒体 artifact、checkpoint、取消、并发容量和结果持久化。
- 保留外部视频导入路径；它不调用 Provider，也不属于本批请求预览合同。
