# 章节与画布视频 Provider 请求预览伪安全清理记录

日期：2026-08-14

## 结论

章节视频和自由画布的视频 Provider 请求预览均为活跃生产功能，不能删除。BFS 发现真正的冗余位于预览专用软化链：无效提示词会被替换为长篇占位文本，缺失结构化元数据会被伪造为完整 shot/frame JSON，缺失或越权图片会被替换为可通过 Provider 请求组装的假 URL。最终展示的因此不是当前输入会产生的真实请求，而是生成路径永远不会接受的第二套请求。

本批删除上述预览专用伪安全逻辑，使章节视频预览与生成复用同一套提示词、元数据、图片绑定、Prompt 长度和 Provider 能力合同。自由画布继续复用同一个 Runtime 图片解析入口；无效图片现在直接失败。合法图片仍使用脱敏 URL 表达请求形状，避免在预览结果中返回真实二进制、签名 URL 或远端临时上传结果。

未新增兜底、兼容、回退、占位、适配器或第二套校验。

## BFS 调用拓扑

```text
章节视频记录页“预览生成请求”
  -> handlePreviewGenerateVideoRequest
  -> previewChapterVideoRequest
  -> preview_chapter_video_request Host command
  -> Server dispatch / Tauri generic dispatch
  -> Runtime preview_chapter_video_request
  -> Orchestrator preview_chapter_video_request_validated
  -> prepare_chapter_video_generation(Preview)
  -> chapter_video_request_rules 唯一 Provider 合同
  -> RuntimeVideoProviderHostPort
  -> sidecar 或 embedded provider 的真实 request body builder

自由画布“预览节点请求”
  -> previewCanvasNodeRequest
  -> preview_canvas_node_request Host command
  -> Runtime canvas workflow preview
  -> preview_canvas_video_request
  -> Orchestrator preview_canvas_video_request_validated
  -> prepare_canvas_video_request(Preview)
  -> RuntimeVideoProviderHostPort
  -> 与章节视频共享的图片校验与脱敏请求 URL

真实生成兄弟路径
  -> run_chapter_video_generation_validated / run_canvas_video_node_validated
  -> 相同提示词合同、图片归属合同和 Provider 能力合同
  -> 仅媒体传输阶段改为真实 data URL / public URL / remote temp upload
```

## BFS 搜索覆盖范围

### 第 0 层：用户入口

- `frontend/src/pages/novel-detail/video/useVideoRecordActions.ts`
- `frontend/src/pages/VideoStudioCanvas.tsx`
- `frontend/src/pages/video-studio/CanvasNodeContentPreviewDialog.tsx`

章节视频预览可携带当前编辑区草稿；自由画布要求先保存节点再预览。两者都是真实用户操作，不是测试或开发入口。

### 第 1 层：前端 API 与类型合同

- `frontend/src/api/video/records.ts`
- `frontend/src/api/video/records.test.ts`
- `frontend/src/api/canvas.ts`
- `frontend/src/api/generated/host.ts`
- `crates/a4n-api-contract/src/video.rs`
- `crates/a4n-api-contract/src/command_manifest.rs`

保留 `PreviewChapterVideoPromptOverrideInput`：它用于预览未保存草稿，但草稿现在必须通过与真实生成相同的严格校验，不再由后端补齐。

### 第 2 层：Host 路由与分发

- `crates/a4n-server/src/routes/commands/dispatch/video.rs`
- `crates/a4n-server/src/routes/commands/media/video_generation/preview.rs`
- `crates/a4n-server/src/routes/commands/media/video_generation/input.rs`
- `crates/a4n-server/src/routes/commands/canvas.rs`
- `crates/a4n-tauri/src/command_registry.rs`
- `crates/a4n-tauri/src/commands/dispatch.rs`

Server 显式解析章节视频预览覆盖字段；Tauri 通过统一命令清单和 dispatch 进入相同 Runtime action。未发现桌面专用分叉合同。

### 第 3 层：Runtime action 与共享 Host Port

- `crates/a4n-runtime/src/actions/video_generation/chapter.rs`
- `crates/a4n-runtime/src/actions/video_generation/canvas.rs`
- `crates/a4n-runtime/src/actions/canvas_workflow/preview.rs`
- `crates/a4n-runtime/src/actions/canvas_workflow/preview/video.rs`
- `crates/a4n-runtime/src/actions/video_generation/provider_host.rs`

`RuntimeVideoProviderHostPort` 同时实现章节视频和画布视频 Host Port。原图片预览软化链会同时影响两个系统，因此必须横向收敛，不能只修章节视频调用点。

### 第 4 层：Orchestrator 编排与 Provider 规则

- `crates/a4n-orchestrator/src/video/chapter_video_generation.rs`
- `crates/a4n-orchestrator/src/video/chapter_video_request_rules.rs`
- `crates/a4n-orchestrator/src/video/provider_prompt_text_rules.rs`
- `crates/a4n-orchestrator/src/video/provider_request_rules.rs`
- `crates/a4n-orchestrator/src/canvas/video_generation.rs`

确认预览和生成原本在同一个 preparation 函数中分叉。删除软化分支后，两者对提示词文本、结构化元数据、图片需求、Provider 图片能力和 Prompt 长度使用唯一合同。

### 第 5 层：图片记录、权限与媒体传输

- `crates/a4n-runtime/src/actions/video_generation/media/reference_images/preview.rs`
- `crates/a4n-runtime/src/actions/video_generation/media/reference_images/provider.rs`
- `crates/a4n-runtime/src/actions/video_generation/media/reference_images/records.rs`
- `crates/a4n-db/src/video/generated_images/references/validation.rs`
- `crates/a4n-orchestrator/src/video/provider_image_rules.rs`

预览现直接复用 `validated_reference_images_for_ids`，由 DB owner 检查记录存在、项目/根文本项目归属和章节归属，再检查已保存文件路径的图片类型。只有全部有效后才生成脱敏请求 URL。生成路径继续额外读取文件并执行真实传输。

### 第 6 层：Provider 请求体与隐式依赖

- `crates/a4n-runtime/src/sidecar_clients/`
- `crates/a4n-video-providers/`
- `crates/a4n-service-protocol/`
- Runtime profile 与 `embedded-media-providers` feature

请求体由 sidecar 或 embedded provider 的正式 builder 生成。仓库未发现反射调用、动态代理、AOP、XML、注解路由或 DI 容器隐藏绑定；Host Port 为 Rust 静态 Trait 实现，feature 分支已纳入全目标编译。

## 入向引用闭包

- 两个章节视频预览占位常量仅被预览文本补齐函数使用。
- `ensure_chapter_video_prompt_preview_texts`、`preview_model_text_or_placeholder`、`replace_with_preview_placeholder` 只构成内部软化链。
- `ensure_chapter_video_prompt_preview_metadata` 及 shot/frame 占位构造器只服务预览，不参与保存或生成。
- `build_video_generation_provider_preview_prompt_with_renderer` 只有章节视频预览调用；真实生成使用同名非 preview builder。
- `allow_preview_placeholders` 只用于上述分叉，其他所有调用恒传 `false`。
- Runtime 图片预览 notes、`ReferenceImagePreviewPlaceholders` 和 Canvas `provider_input_note` 只承载“无效但继续”的软化结果。
- `load_reference_images_by_id` 与 `reference_image_project_ids_in_reference_order` 在软化链删除后只剩专用测试调用。

## 优化记录

1. 删除章节视频 visual prompt 预览占位常量。
2. 删除章节视频 motion prompt 预览占位常量。
3. 删除预览文本自动补齐入口。
4. 删除违规文本转占位文本分派器。
5. 删除通用占位替换与 note 拼接 helper。
6. 删除缺失 `metadata_json` 时伪造空对象的路径。
7. 删除 metadata 非对象时伪造空对象的路径。
8. 删除 `metadata.shot` 自动补齐 helper。
9. 删除 shot 缺失章节自动补齐 helper。
10. 删除占位 shot JSON 构造器。
11. 删除占位 frame JSON 构造器。
12. 删除 12 组 shot 章节占位 fixture。
13. 删除独立的 Provider preview prompt builder。
14. 删除 generate/preview 双模式内部 prompt builder。
15. 删除 `build_provider_prompt_plan` 的预览占位开关。
16. 删除公共变量构造器的预览占位开关。
17. 删除 Provider 文本校验入口的预览占位开关。
18. 删除上下文文本校验入口的预览占位开关。
19. 删除违规检测器的预览占位开关，预览占位文本始终视为无效。
20. 章节视频预览改为复用真实生成的唯一 Provider prompt builder。
21. 预览草稿中的空、过短、旧格式、字典化或内部流程文本改为显式失败。
22. 删除 Prompt 长度违规“预览记 note 后继续”的路径。
23. 删除 Provider 图片绑定违规“预览记 note 后继续”的路径。
24. 删除通用 generate/preview 软化分派 helper。
25. 首帧串联起始图缺失时，预览与生成统一失败。
26. 分镜图缺失时，预览与生成统一失败。
27. 首尾帧不完整时，预览与生成统一失败。
28. 删除缺图错误转预览 note helper。
29. 删除章节图片解析结果中的 `notes` 第二通道。
30. 删除批量追加图片软化 note helper。
31. Runtime 图片预览改为复用正式 DB 图片归属校验。
32. 删除不存在图片时伪造 `missing_row=1` URL 的路径。
33. 删除越权项目图片仅记 note 后继续的路径。
34. 删除跨章节图片仅记 note 后继续的路径。
35. 删除空文件路径仅记 note 后继续的路径。
36. 删除不支持图片类型仅记 note 后继续的路径。
37. 删除 `size unknown` 和 `validation=deferred` URL 元数据兜底。
38. 将合法图片预览 URL 明确收敛为脱敏请求形状，不携带真实媒体内容。
39. 删除 `ReferenceImagePreviewPlaceholders` 包装结构。
40. 删除自由画布图片解析结果的 `provider_input_note`。
41. 删除自由画布请求预览结果的 `provider_input_note`。
42. 删除自由画布 preparation 中的 note 字段和透传。
43. 删除自由画布生成快照中的恒空 `provider_input_note`。
44. Runtime 画布请求预览不再展示无效输入软化 note。
45. 删除两个验证“无效图片仍返回占位 URL”的旧测试。
46. 删除软化链退役后无调用者的 `load_reference_images_by_id`。
47. 删除只剩自测调用的项目 ID 顺序 helper。
48. 删除上述死 helper 的专用测试与测试再导出。

## 规模

- 修改 15 个既有 Rust 文件，新增 1 个清理记录文件。
- 最终提交涉及 16 个文件，净变化 `+294/-760`。

## 验证记录

- `cargo fmt --all`：通过。
- `timeout 60s cargo check -p a4n-orchestrator -p a4n-runtime --all-targets`：通过，无警告。
- `timeout 60s cargo test -p a4n-orchestrator provider_prompt_text_rules --lib`：1/1 通过。
- Runtime 媒体专项测试首次在 60 秒内仅完成编译并被超时终止；复用现有 `target/` 增量重跑后 21/21 通过。
- `git diff --check`：通过。
- 退役占位常量、补齐函数、软化 helper、图片 notes 包装与只测死函数在 `crates/`、`frontend/`、`prompts/`、`scripts/` 中 `rg` 零命中。

## 保留边界

- 保留章节视频和自由画布两个真实请求预览入口。
- 保留章节视频当前编辑区草稿覆盖能力；覆盖后的草稿必须通过真实生成合同。
- 保留权限、项目/章节归属、执行身份、Provider 能力、时长、画幅和 Prompt 长度校验。
- 保留合法图片的脱敏预览 URL，因为请求预览不得返回真实二进制、签名 URL、JWT 或远端临时上传 artifact。
- 保留生成路径的 data URL、public URL 和 remote temp upload 三种真实媒体传输。
- 保留历史 Provider 请求快照中的媒体省略标识；它描述已发生请求的脱敏快照，不是无效输入 fallback。
- `preview_reference_media_video_request` 是另一条活跃功能链，其 prompt、token 和 URL 占位逻辑需从自身入口独立 BFS 后处理，本批不跨路径套用结论。
