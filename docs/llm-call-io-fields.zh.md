# 生产 LLM 调用点的输入/输出字段清单

> 核对基线：`debug`，HEAD `53b4520885cc`，核对日期 2026-10-05。这是源码字段清单，不是运行时请求样本；敏感字段只列字段名，不列值。

## 统计口径

按业务层把请求或专用输入交给 LLM driver 的静态入口表达式计数，共 **49 处**；一个源码位置在循环、Agent 多轮、重试中可以运行多次，这些运行次数不重复计数。共享 driver/provider 内部的转发不重复计数；复用型 driver 按具体业务调用位置拆开。

之前的 31 处统计把 4 个复用型 driver 各计成了一个位置，并漏掉了大纲和立绘流程。修正口径为 `31 - 4 + 4（WorkflowLlmOnce）+ 4（Galgame pipeline）+ 9（Manga pipeline）+ 3（拍摄剧本）+ 5（大纲/立绘）= 52`。

配置连通性探测单列 1 处。图片/视频生成 provider 也有两个 `/chat/completions` 请求，但它们使用媒体生成合同，不计入这 49 个聊天 LLM 入口；字段见 2.11。

## 1. 应用请求、上下文组装与传输边界

### 1.1 请求：`ChatRequest`

模型调用的统一应用层请求类型见 `crates/llm/a4n-llm-provider-contract/src/provider/request.rs:10`。各业务调用传入的是此结构；上游业务 DTO 会先被转换成 `messages`、图片和可选控制参数。它不是供应商 HTTP 请求体：运行时还会完成预算规划、上下文包选取和协议适配，见 1.4–1.5。业务 DTO、manifest 候选包、实际消息和供应商请求体应分开核对。

| 字段 | 类型 | 应用层含义/行为 |
| --- | --- | --- |
| `messages` | `Vec<ChatMessage>` | 必填消息数组。 |
| `vision_images` | `Vec<ChatImage>` | 默认空数组；非空时每项为 `media_type`、`data_base64`。 |
| `temperature` | `Option<f32>` | 可选采样温度。 |
| `max_tokens` | `Option<usize>` | 可选输出上限。 |
| `response_format` | `Option<ResponseFormat>` | 可选；`ResponseFormat` 只有 `type` 字段，JSON 模式值为 `json_object`。 |
| `stream_include_usage` | `bool` | 默认 `false`；请求流尾部 usage。 |
| `cache_scope_key` | `Option<String>` | Prompt cache 的请求范围键。 |
| `cache_breakpoints` | `Vec<PromptCacheBreakpoint>` | 默认空；每项字段为 `message_index`、`byte_offset`。 |
| `cache_mode` | `Option<PromptCacheMode>` | `explicit` 或 `implicit`。 |
| `tools` | `Option<Vec<Value>>` | 可选工具定义；schema 是 JSON 值数组。 |
| `tool_choice` | `Option<String>` | 可选工具选择策略。 |
| `stop_sequences` | `Option<Vec<String>>` | 可选停止序列。 |
| `trace_context` | `Option<LlmTraceContext>` | `serde(skip)`，不进入 provider 请求体；用于 A4N trace/checkpoint。 |
| `context_manifest_json` | `Option<Value>` | `serde(skip)`，不进入 provider 请求体；仅内部上下文/快照使用。 |

应用结构的 serde 序列化规则：`vision_images`、`cache_breakpoints` 为空时省略；`stream_include_usage=false` 省略；`response_format`、`cache_scope_key`、`cache_mode` 为 `None` 时省略；`trace_context`、`context_manifest_json` 始终跳过普通序列化。`temperature`、`max_tokens`、`tools`、`tool_choice`、`stop_sequences` 没有 `skip_serializing_if`，值为 `None` 时序列化为 `null`。`ChatMessage.tool_calls`、`tool_call_id` 为 `None` 时省略。该规则也适用于字段对应的 LLM sidecar 合同，但不能据此推断供应商 HTTP 中也发送这些 `null`；供应商适配见 1.5。内部 `snapshot_value()` 会另外保留 trace 和 manifest，用于恢复与审计。

`ChatMessage {role: MessageRole, content: String, tool_calls?: Vec<ToolCall>, tool_call_id?: String}`。`role` 在应用/sidecar 合同中为 `system`、`user`、`assistant`、`tool`。`ChatImage {media_type: String, data_base64: String}`。`ToolCall {id: String, name: String, arguments: Value, stream_index?: u32, provider_metadata?: Value}`。供应商适配可改变消息角色、content 形式和工具调用包装。

`LlmTraceContext` 的字段为 `project_id?`、`root_project_id?`、`workflow`、`step_key`、`generation_run_id?`、`workflow_run_id?`、`call_key?`、`input_fingerprint_sha256?`、`context_plan_json?`、`context_pack_refs_json?`。它用于审计/成本观测，不透传到 OpenAI-compatible 请求体。

### 1.2 Sidecar LLM 请求封套

经 LLM sidecar 时，provider payload 是 `LlmGatewayPayload<LlmChatRequest>`：

- `provider`: `provider: String`、`base_url: String`、`model: String`、`api_key: Option<String>`、`timeout_secs: u64`、`extra_body_json: Option<Value>`。
- `request`: `LlmChatRequest`。字段与 `ChatRequest` 的普通 serde 序列化字段一致，但不含内部字段 `trace_context`、`context_manifest_json`；这是 A4N 到 sidecar 的请求合同，sidecar 内还会适配成供应商 HTTP 请求。
- 两个 nullable 配置字段 `api_key`、`extra_body_json` 在 sidecar 合同中要求显式出现；值可以为 `null`。凭证值不应进入文档、日志或请求样本。
- 外层 `ServiceRequest` 字段为 `request_id`、`trace_context`、`schema_version`、`user_id?`、`project_id?`、`root_project_id?`、`runtime_config_snapshot?`、`secret_policy`、`capability_version`、`payload`。服务 trace 的 `TraceContext` 字段为 `trace_id`、`parent_span_id?`。
- 嵌套合同：`schema_version={protocol,database?}`；`secret_policy={scope,allow_system_secret,allow_user_secret,allow_upload_token}`，`scope=none/local_sidecar_only/trusted_remote`。普通 `ServiceResponse<T>={status,trace_id?,usage?,artifact_refs[],payload?,error?}`；`error={code,message,retryable?}`。
- LLM stream 使用 SSE `CapabilityTaskEvent={task_id,event_kind,sequence,error?,payload?}`；`event_kind=progress/chunk/completed/failed`，chunk 的 payload 是 `LlmStreamChunk`。连通性 probe 的 payload 是 `LlmProbeOutput={response_preview,usage,finish_reason}`；`usage` 和 `finish_reason` 值可为 `null`，但键必须出现。

### 1.3 模型流和聚合响应

`ChatChunk` 以及 sidecar 的 `LlmStreamChunk` 字段相同：

- `content: Option<String>`
- `reasoning_content: Option<String>`
- `tool_calls: Option<Vec<ToolCall>>`
- `finish_reason: Option<String>`
- `index: u32`
- `usage: Option<UsageInfo>`

`UsageInfo {prompt_tokens: usize, completion_tokens: usize, total_tokens: usize, prompt_cache_hit_tokens?: usize, prompt_cache_miss_tokens?: usize, prompt_cache_write_tokens?: usize}`。`ChatResponse {message: ChatMessage, usage?: UsageInfo, finish_reason?: String, reasoning_content?: String}`。

| 包装器/结果 | 输入字段 | 输出字段 |
| --- | --- | --- |
| `LlmGenerationRequest` | `request`、`stream_id`、`cancellation`、`checkpoint? {writer, session, metadata}`、`usage_base?` | `LlmGenerationOutput {content, usage?, finish_reason, content_sha256}` |
| `ChatJsonStreamInput` | `llm`、`request`、`prompt_name`、`truncated_message`、`structured_progress?`、`cancel_token?`、`cancelled_message` | `StructuredLlmStreamResult {response_content, usage?, finish_reason?}`；这是原始文本，不是已解析 JSON。 |
| `VideoPromptRenderInput` | `service`、`prompt_renderer`、`prompt_name`、`project_id`、`variables`、`context_manifest`、`temperature`、`llm_call_trace {workflow_run_id, call_key, input_fingerprint_sha256}`、`cancel_token?`、`step`、`progress`、`chunks`、`structured_progress?` | `RenderedVideoPromptTrace {prompt_name, request, response_text, usage?, finish_reason?}` |
| Agent Host `chat_stream` | `user_id`、`snapshot?`、`config_id?`、`request` | `AgentChatStream = Stream<Result<ChatChunk, AppError>>` |

Sidecar 每个 chunk 外包 `CapabilityTaskEvent {task_id, event_kind, sequence, error?, payload?}`；`event_kind` 为 `progress`、`chunk`、`completed`、`failed`。`chunk` 的 `payload` 是 `LlmStreamChunk`。连接心跳是 `progress` 事件，模型流结束是 `completed`。

使用 generation task 事件的调用会再映射为 `GenerationTaskEvent {schema_version: u16, task_id: String, attempt: u32, sequence: u64, emitted_at_ms: i64, event}`。其中 `TextDeltaEvent {stream_id, offset, delta, chunk_index}`；`UsageEvent {usage, cumulative}`。`GenerationUsage` 与 `UsageInfo` 字段名相同，字段均为 `u64` 或可空 `u64`。

### 1.4 上下文 manifest

`LlmContextManifest {schema_version, packs[], dynamic_user_content, output_contract?}`；每个 pack 是 `{scope_kind, pack_kind, source_refs_json?, config_hash, required, stable_prefix, variants[]}`；variant 是 `{compression_level, canonical_content_json}`。`packs` 是候选上下文，不表示所有包和所有变体都会发送。

实际组装流程见 [运行时上下文准备](../crates/host/a4n-runtime/src/dynamic_llm/trace/preparation.rs#L17)、[预算选取](../crates/llm/a4n-llm-context/src/planner/budget/selection.rs#L18) 和 [消息渲染](../crates/llm/a4n-llm-context/src/manifest/render.rs#L16)：

1. 统计 system、动态消息和工具定义的输入开销，并为输出保留 token；为各包建立可选变体。
2. 每个包最多选一个能放入窗口的变体；可选包放不下时记录为 dropped，必需包连最小变体也放不下时显式报错。manifest 中存在某字段，不等于最终选择的变体仍含该字段。
3. 将选中内容依次渲染为 `[稳定上下文]` → `[输出契约]` → `[可变上下文]` → `[当前任务]`；空部分省略。包的来源引用、配置摘要等元数据不会自动成为模型文本，除非业务另将它们写入内容。
4. manifest 只重写首条非 system 消息，且该条必须是 user；没有时追加 user。其余历史、assistant/tool 消息保留，不能用单条 user manifest 覆盖它们。[重写边界](../crates/llm/a4n-llm-context/src/request/rewrite.rs#L6)

`context_aware_system_user_request` 的初始消息只渲染动态任务和独立的输出契约；上下文包在运行时选取、填入。业务准备阶段的 `messages` 快照因此不能单独证明模型最终看到了哪些 pack。最终选择记录在 `trace_context.context_plan_json` 和 `context_pack_refs_json` 中；这些记录和 manifest 本身不进入供应商请求体。Agent 主回合由自己的历史投影器组装消息，`context_manifest_json=None`，不走 manifest 重写。

### 1.5 供应商 HTTP 请求适配

运行时还会根据模型配置解析 `max_tokens`、配置 prompt cache，并在普通 LLM 调用中设置 `stream_include_usage=true`，见 [请求准备](../crates/host/a4n-runtime/src/dynamic_llm/provider/preparation.rs#L26)。下面列的是当前项目适配器实际构造的字段，不把应用结构直接当作 HTTP body。

| 协议/实现 | 实际请求形状与差异 |
| --- | --- |
| [OpenAI-compatible Chat Completions](../crates/llm/a4n-llm-http/src/provider/request.rs#L75) | 顶层含 `model,messages,n:1`；流式调用加入 `stream:true`，需要 usage 时加入 `stream_options.include_usage`。`temperature,response_format,tools,tool_choice` 仅在有值时写入；`stop_sequences` 映射为 `stop`。输出上限写为 `max_tokens`；o1/o3/o4 系列改用 `max_completion_tokens` 并省略通用 temperature。 |
| Chat Completions 消息/图片/缓存 | 工具调用变为 `{id,type:"function",function:{name,arguments:JSON字符串}}`；不是直接发送应用层 `{id,name,arguments:Value}`。`vision_images` 附到最后一条 user 的 `content`，形成 text/image_url parts，不作为同名顶层字段发送。`cache_scope_key` 映射为 `prompt_cache_key`，cache mode/断点由适配器编码。启用推理回放时，assistant 工具消息还可带 `reasoning_content`。[图片与缓存适配](../crates/llm/a4n-llm-http/src/cache.rs#L28) |
| [OpenAI Responses](../crates/llm/a4n-llm-http/src/native/responses.rs#L9) | 使用 `input[]`，工具调用/结果为 `function_call` / `function_call_output`；图片使用 `input_image`。输出上限为 `max_output_tokens`，JSON 模式为 `text.format.type=json_object`；设置 `stream:true,store:false,include:[reasoning.encrypted_content]`。非空 `stop_sequences` 被明确拒绝。 |
| [Anthropic Messages](../crates/llm/a4n-llm-http/src/anthropic.rs#L61) | system 文本放顶层 `system[]`；其余消息使用 content blocks，工具调用为 `tool_use`，工具结果映射为 user 中的 `tool_result`。工具 schema 为 `input_schema`；图片为 base64 source block；缓存为块级 `cache_control`。JSON 模式通过追加 system 输出指令表达，不发送应用层 `response_format`。 |
| [Gemini GenerateContent](../crates/llm/a4n-llm-http/src/native/gemini.rs#L10) | 使用 `systemInstruction,contents[].parts,generationConfig`；assistant 角色映射为 `model`，工具为 `functionCall/functionResponse`，图片为 `inlineData`。输出上限、停止序列和 JSON 模式分别映射到 `maxOutputTokens,stopSequences,responseMimeType`；工具定义为 `tools[].functionDeclarations`，调用策略为 `toolConfig.functionCallingConfig`。 |

`extra_body_json` 经校验后由各适配器合并；不允许覆盖的字段由对应实现拒绝。工具的 `provider_metadata` 也不是通用 HTTP 字段，适配器只按协议回放其需要的推理/签名数据。

## 2. LLM driver 家族清单（原 31 项）

下表保留原 driver 家族的输入/输出摘要；其中复用型调用 #8、#16、#18、#19 是共享执行器位置。它们的具体业务入口在 2.2–2.5 展开，计数时由展开后的调用点替换，不重复相加；旧清单漏掉的大纲/立绘入口列于 2.6。应用请求字段见 1.1，供应商适配见 1.5，其余业务输入展开见 2.12。

### A. Generation stream driver：7 处

这 7 处都传 `LlmGenerationRequest`，模型原文由 `LlmGenerationOutput` 返回；generation driver 另发文本、reasoning、usage 和 progress 事件。

| # | 位置 | 请求内容 | 输出字段/消费方式 |
| --- | --- | --- | --- |
| 1 | [章节正文生成](../crates/host/a4n-orchestrator/src/chapter_generation/application/generation.rs#L251) | 已准备的章节 `ChatRequest`，含章节上下文、来源/RAG 及输出设置；`stream_id="main"`。 | `content` 是正文；`usage`、`finish_reason`、`content_sha256` 用于任务/检查点；正文 delta 走 `main` 流。 |
| 2 | [蓝图优化](../crates/host/a4n-orchestrator/src/blueprint/refine/execution/call.rs#L103) | `PreparedBlueprintRefine.request`，即当前蓝图优化 prompt。 | 输出文本解析为 `ParsedBlueprintOptimization {raw, style?, full_synopsis?, world_setting?, characters?, relationships?}`；调用结果另带 `usage?`、`finish_reason`。 |
| 3 | [局部补丁生成](../crates/host/a4n-orchestrator/src/optimize/patch/generation/execution/call/transport.rs#L29) | `PatchGenerationCallInput.request`，包含待改目标和补丁指令。 | `LlmGenerationOutput.content` 被包装为 `ChatResponse.message.content`；`usage`、`finish_reason` 保留，`reasoning_content=None`。 |
| 4 | [蓝图阶段生成](../crates/host/a4n-orchestrator/src/blueprint/generation/stage/transport.rs#L32) | 阶段专属 `ChatRequest` 和 `BlueprintStageSpec`；流 ID 为 `raw_json`。 | `BlueprintStageLlmOutput {response_content, usage?, finish_reason?}`；上层按该阶段 schema 检查 JSON/截断。 |
| 5 | [优化 Agent 决策](../crates/host/a4n-orchestrator/src/optimize/agent_loop/executor/decision/execution/call.rs#L148) | `chat_request`，由 system prompt、基础 user message、observation、step 和当前决策上下文构造。 | 模型 JSON 必须是 `{thought, public_update?, action, arguments}`；`action` 为 `think/run_tool/final`，`arguments` 必须为对象。内部结果为 `{response, decision, action, restored}`。 |
| 6 | [章节视觉规划](../crates/host/a4n-orchestrator/src/chapter_visual_plan/generation.rs#L326) | `StartVisualPlanPromptGenerationInput {meta, project_id, source_chapter_id, record_id, expected_revision, segment_id, user_description}`，再由服务补充章节/片段上下文生成 `ChatRequest`。 | `content` 写入 `candidate.prompt`；验证非空且不超过 100,000 字符后持久化候选。 |
| 7 | [灵感对话](../crates/host/a4n-orchestrator/src/inspiration/application.rs#L213) | `StartInspirationGenerationInput {meta, project_id, message}` 加服务读取的对话历史和渲染 prompt。 | `content` 必须是 `InspirationResponse` JSON：`ai_message`、`next_question`、`progress_summary`、`ui_control {type, options?, placeholder?}`、`conversation_state`、`is_complete`。`options[]` 含 `id`、`label`、`description`、`key_elements`；完成态还要求 `conversation_state.chapter_count`、`genre`。 |

### B. 返回 `ChatResponse` 的业务包装器：11 个旧 driver 家族项

这些调用的返回类型都是 `ChatResponse {message, usage?, finish_reason?, reasoning_content?}`，但当前统一 stream 包装器只回填主文本、usage 和归一化 finish reason，`message.tool_calls=None`、`reasoning_content=None`。类型能容纳这些字段，不代表该路径实际保留了它们；详见第 4 节。下表列出 `message.content` 的业务 schema 和被消费字段。

| # | 位置 | 请求内容 | 输出字段/消费方式 |
| --- | --- | --- | --- |
| 8 | [工作流单次 LLM](../crates/host/a4n-orchestrator/src/workflow/llm_once/execution.rs#L36) | `system_prompt`、`user_prompt`；trace 含 `project_id`、`root_project_id`、`workflow`、`step_key`、`workflow_run_id`、`call_key`、输入 fingerprint。 | `message.content` 是严格 JSON；调用者传入 `validate(Value)->T`，返回 `(Value,T)`。没有统一的 `T` 字段集合。 |
| 9 | [导入反推](../crates/host/a4n-orchestrator/src/import_reverse/execution/call.rs#L44) | `PreparedImportReverseCall {target, request, workflow_run_id, fingerprint_json}`；`target` 为 `blueprint/outline/part_outline`。 | `message.content` 是 target-specific JSON，经对应 validator 验证后放入候选 `result: Value`；候选封套还含 `workflow_run_id`、`target`、`state`、`warnings`、`applied?`。 |
| 10 | [视频提示词优化](../crates/host/a4n-orchestrator/src/video/prompts/optimization.rs#L380) | 原提示词记录、`prompt_kind`、`prompt_output_language` 和优化 `ChatRequest`。 | JSON 字段：`visual_prompt: string`、`motion_prompt: string\|null`、`negative_prompt: string\|null`；三键均须存在，随后按提示词类型检查 nullable 规则。 |
| 11 | [场景正文生成](../crates/host/a4n-orchestrator/src/chapter_scene/execution.rs#L265) | 带 scene 目标、章节上下文和 call trace 的 `ChatRequest`。 | `message.content` 为正文字符串；`normalize_scene_generation_output` 后保存/返回 `String`。 |
| 12 | [角色冲突检查](../crates/host/a4n-orchestrator/src/character/conflict/execution.rs#L231) | `CharacterConflictCheckInput {project_id, name, start_chapter?, end_chapter?}` 派生的请求，并带 source ID 集合。 | JSON 数组元素字段：`chapter_number`、`field`、`expected`、`actual`、`expected_source_id`、`actual_source_id`、`severity`；severity 为 `minor/moderate/major`。 |
| 13 | [角色档案同步阶段](../crates/host/a4n-orchestrator/src/character/profile_sync/stages/execution.rs#L260) | `CharacterProfileSyncStageInput {project_id, workflow_run_id?, profile_id, profile_name, start_chapter, target_chapter, selected_chapter_numbers, chapter_data, baseline_state}` 加 `previous_steps`。 | 响应原文、usage、结束原因保存在 `CharacterProfileSyncStageCallResult`；JSON 根键依阶段分别为 `events`、`attribute_changes`、`chapter_states`、`behaviors`、`deletion_candidates`。字段细则见本节后表。 |
| 14 | [隐性属性复审](../crates/host/a4n-orchestrator/src/character/implicit_review/execution.rs#L340) | `ImplicitAttributeReviewSummary {attribute_key, current_value, conform_count, non_conform_count, conform_rate, threshold_reached, evidence[]}`。 | JSON 必须含 `decision`（`keep/modify/delete`）、`reasoning`、`new_value`、`supporting_behavior_ids`；keep/delete 时 `new_value=null`，modify 时非 null。系统根据行为 ID 派生 `evidence_summary`。 |
| 15 | [隐性属性分类](../crates/host/a4n-orchestrator/src/character/implicit_classification/execution.rs#L142) | 行为记录请求和当前 `implicit_attributes` 对象。 | JSON 顶层键固定为 `classifications`、`reasoning`、`suggested_new_attributes`；`classifications` 是以现有属性 key 为键、值为 `conform/non-conform` 的对象；suggestion 元素为 `{key, value}`。 |
| 16 | [Galgame 阶段](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/llm/stage/execution.rs#L39) | `prepared.request_messages`、stage、workflow/call trace；请求设置 `response_format.type="json_object"`。 | 严格 JSON 解析后由该 stage 的 `validate(Value)->T` 检查，返回 `(Value,T)`；不同阶段没有一个全局固定 `T`。 |
| 17 | [漫画提示词优化](../crates/host/a4n-orchestrator/src/manga/prompt_optimization/execution.rs#L160) | 原漫画提示词及优化约束组成的 `ChatRequest`。 | 严格 JSON `{image_prompt: string}`；最终返回 `image_prompt` 字符串。 |
| 18 | [漫画流程提示词](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/llm/execution.rs#L115) | `MangaRenderTraceInput {prompt_name, project_id, workflow_run_id, call_key, call_order, variables, user_content, temperature, cancel_token, stream_id, stream_reporter, restore_from_checkpoint}`。 | JSON 由每个 prompt 的 `validate(Value)->T` 校验；返回 trace `{prompt_name, request_json, messages_json, response_text, workflow_run_id, call_key, call_order, input_fingerprint_json, usage?, finish_reason?}` 和 `T`。 |

#### 角色档案同步五个阶段的 JSON 字段

| 根键 | 元素字段/约束 |
| --- | --- |
| `events[]` | `chapter_number`、`event_type`、`summary`、`related_characters[]`、`evidence_anchor_id`。 |
| `attribute_changes[]` | `assertion`（`observed`，或仅 `implicit` 类别可用 `inferred`）、`chapter_number`、`category`（`explicit/implicit/social`）、`key`、`operation`（仅 `add/modify`）、非 null 的 `new_value`、`change_description`、`event_cause`、`evidence_anchor_id`。`old_value`、`evidence` 不允许由模型提供；删除提议只能进入 `deletion_candidates`。 |
| `chapter_states[]` | `chapter_number`、`state_summary`、`current_location?`、`current_goal?`、`emotional_state?`；系统要求覆盖本次选定章节。 |
| `behaviors[]` | `chapter_number`、`behavior_type`、`behavior_description`、`original_text_anchor_id`、`behavior_tags[]`、`related_characters[]`、`impact_analysis?`。`original_text`、`classification_results` 不由该阶段输出。 |
| `deletion_candidates[]` | `chapter_number`、`category`、`key`、`reason`、`evidence_anchor_id`。 |

### C. 结构化流：5 个旧 driver 家族项

`chat_json_stream_result_with_sink` 的输入类型为 `ChatJsonStreamInput`，输出只含 `response_content`、`usage?`、`finish_reason?`。它不会返回已解析 JSON，也没有 `reasoning_content` 或 `tool_calls` 字段。

| # | 位置 | 请求内容 | 输出字段/消费方式 |
| --- | --- | --- | --- |
| 19 | [拍摄剧本原始调用](../crates/host/a4n-orchestrator/src/shooting_script/stage/raw_call.rs#L72) | `workflow_run_id`、`call_key`、`call_order`、fingerprint、`ChatRequest`、`label`、结构化进度、取消/流回调。 | `RawCallResult {response_content, restored, restored_parsed_output?, response_only_checkpoint, usage?, finish_reason?, upgraded_contract}`；model JSON 是 stage/call_key 专属。 |
| 20 | [角色发现](../crates/host/a4n-orchestrator/src/character/discovery/execution.rs#L264) | API 输入 `DiscoverCharacterProfilesInput {project_id, chapter_number?}`；消息包含已选章节、证据锚点和现有角色上下文。 | 模型 JSON 根键 `characters`。每项键为 `name`、`aliases`、`importance`（`primary/secondary/minor`）、`role`、`explicit_attributes`、`implicit_attributes`、`social_attributes`、`evidence_anchor_id`。系统派生章节范围、摘要和证据文本。API `created[]` 字段为 `id`、`project_id`、`name`、`gender?`、`age?`、`base_attributes`、`current_state`、`character_arc?`、`voice_fingerprint?`、`appearance?`、`background?`、`personality?`、`last_synced_chapter`、`attribute_counts {explicit, implicit, social}`、`created_at`、`updated_at`；`existing[]` 为 `name`、`aliases`、`reason`。结果顶层另有 `status`、`chapters_analyzed`、`chapter_numbers`、`discovered_count`。 |
| 21 | [场景档案同步阶段](../crates/host/a4n-orchestrator/src/scene_profile/application/stages/execution/call.rs#L37) | 当前章节、既有场景/角色/物体资料、文本锚点和 stage `ChatRequest`。 | 根键 `scenes`；每项允许键：`scene_ref`、`scene_profile_id?`、`name`、`aliases`、`scene_type`、`importance`、`parent_scene_id?`、`base_description`、`visual_description`、`spatial_layout`、`atmosphere`、`entry_points`、`rules`、`evidence_anchor_ids`。系统派生章节范围、证据文本和 `image_prompt_seed`。 |
| 22 | [章节分析阶段](../crates/host/a4n-orchestrator/src/chapter_analysis/stages/runner/call.rs#L36) | 章节正文/分析上下文、stage `ChatRequest`、`ChapterAnalysisStageSpec {stage, step_key, step_order, context, restored_progress, fields, progress_span}`。 | `StructuredLlmStreamResult` 原文由阶段解析器处理。进度所跟踪的根键：事实 `characters/locations/items/event_candidates/foreshadowing_candidates/world_facts`；摘要 `metadata_additions/summaries`；角色状态 `character_states`；伏笔 `foreshadowing/key_events`。 |
| 23 | [物体发现](../crates/host/a4n-orchestrator/src/object_discovery/execution.rs#L243) | API 输入 `ObjectDiscoveryInput {project_id, chapter_number?}`；消息包含章节、文本锚点和既有资料。 | JSON 根键 `objects`。每项允许键：`name`、`aliases`、`object_type`、`importance`、`visual_description`、`material_and_state`、`story_usage`、`visual_usage`、`generation_usage`、`location_context?`、`evidence_anchor_ids`。`first_seen_chapter`、`last_seen_chapter`、`evidence`、`image_prompt_seed` 由系统派生。 |

### D. 直接使用统一响应包装器：1 处

| # | 位置 | 请求内容 | 输出字段/消费方式 |
| --- | --- | --- | --- |
| 24 | [正文事实归纳](../crates/host/a4n-orchestrator/src/fact_feedback/run.rs#L253) | `PreparedConsolidation.request`，包含当前来源版本和待归纳正文事实。 | JSON `{changes: FactMaterialChange[]}`；每个 change 字段：`material_kind`、`target_id?`、`path`、`operation`、`before`、`after`、`reason`、`source_span_ids[]`。 |

### E. 视频提示词渲染：2 处

| # | 位置 | 请求内容 | 输出字段/消费方式 |
| --- | --- | --- | --- |
| 25 | [新渲染路径](../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/render_restore.rs#L90) | `VideoPromptRenderInput` 的 `prompt_name`、`project_id`、`variables`、`context_manifest`、`temperature` 和 trace/cancel/progress 回调；渲染器生成实际 `ChatRequest`。 | `RenderedVideoPromptTrace {prompt_name, request, response_text, usage?, finish_reason?}`；response JSON 字段由具体 `prompt_name` 的 validator 决定。 |
| 26 | [恢复或渲染路径](../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/render_restore.rs#L270) | 与上一处相同；先查对应 checkpoint。 | 同一 trace schema；checkpoint 命中时不发生新的模型请求。 |

### F. Agent Runtime：2 处

| # | 位置 | 请求内容 | 输出字段/消费方式 |
| --- | --- | --- | --- |
| 27 | [Agent 主回合](../crates/agent/a4n-agent-runtime/src/engine/loop.rs#L757) | `AgentRuntimeHost::chat_stream(user_id, snapshot?, config_id?, request)`；消息由角色/协议/范围/Skill 目录、旧摘要、历史对话与工具 observation、当前目标及执行状态组装。`native_tool_calling` 把 `{type:"function",function:{name,description,parameters}}` 放入顶层 `tools[]`；`json_compat` 不传 tools，而将相同工具定义写入 system prompt 并启用 JSON mode。详见 6.10。 | chunk 字段同 1.3。原生模式聚合 `tool_calls` 或最终文本；JSON 兼容模式解析 `message.content` 中的 `{action,arguments}`，action 为 `think/run_tool/final`，arguments 随动作变化。两种协议不能用同一个模型输出 JSON schema 表示。 |
| 28 | [Agent 上下文压缩](../crates/agent/a4n-agent-runtime/src/engine/context.rs#L99) | user JSON 为 `{previous_summary,current_request:[{message_id,content}],messages:[{message_id,message:ChatMessage,execution_id}],covered_through_sequence}`。`messages` 只取尚未摘要历史中能按完整工具批次装入预算的前缀，保留最新批次给主回合；不重新发送全部历史。 | JSON 字段：`schema_version:2`、`current_goal`、`constraints[]`、`decisions[]`、`facts_resources[]`、`completed[]`、`pending[]`、`tool_state[]`、`covered_through_sequence`；覆盖边界须等于本批输入边界，校验后持久化摘要。 |

### G. Runtime 直接调用：3 处

| # | 位置 | 请求内容 | 输出字段/消费方式 |
| --- | --- | --- | --- |
| 29 | [图片视觉评估](../crates/host/a4n-runtime/src/actions/image_records/reading.rs#L122) | `ReadImageInput {project_id, image_ids[], chapter_id?, instruction?}`；消息含 system/user 文本，图片另由 `vision_images[] {media_type, data_base64}` 发送。 | `ChatResponse.message.content` 被截取为 `analysis`；API 输出 `ReadImageResult {project_id, image_ids, analysis}`。 |
| 30 | [画布文本素材草稿](../crates/host/a4n-runtime/src/actions/canvas_workflow/text_asset_draft.rs#L65) | `CanvasTextAssetDraftInput {project_id, llm_config_id?, instruction, source_asset_id?, source_version?}`；可选 source 的正文加入 user prompt。 | 只消费 `ChatResponse.message.content`；输出 `CanvasTextAssetDraftSummary {content}`。 |
| 31 | [画布文本生成端口](../crates/host/a4n-runtime/src/actions/canvas_workflow/text_nodes/host_port.rs#L35) | `CanvasTextGenerationLlmCall {project_id, workflow_run_id, config_id?, request}`。 | 端口原样返回 `ChatResponse {message, usage?, finish_reason?, reasoning_content?}` 给 Canvas application。 |

## 2.1 修正后的调用点计数

| 类别 | 静态入口数 |
| --- | ---: |
| Generation stream driver | 7 |
| 返回 `ChatResponse` 的业务入口 | 25 |
| 结构化流业务入口 | 7 |
| 正文事实归纳 | 1 |
| 视频提示词 | 2 |
| Agent Runtime | 2 |
| Runtime 直接调用 | 3 |
| 大纲生成 | 3 |
| 立绘提示词 | 2 |
| **合计** | **52** |

复用入口展开为：旧 #8 `WorkflowLlmOnce` 1→4；旧 #16 Galgame pipeline 1→4；旧 #18 Manga pipeline 1→9；旧 #19 拍摄剧本 1→3。另补大纲 3 处和立绘 2 处。每处运行多少次取决于输入、循环、重试与 checkpoint 命中情况。

## 2.2 WorkflowLlmOnce：4 个业务调用点

四处共用 `WorkflowLlmOnceInput {prompt_renderer, project_id, root_project_id, workflow_run_id, workflow, step_key, stage_key, call_order, prompt_name, user_prompt, cancel_token?}`。模型收到 system/user 两条消息，`temperature=0.25`、JSON mode。输出先严格解析 JSON，再由调用点 validator 校验；checkpoint 保存 `parsed_output`、原文、usage、finish reason 和 workflow/prompt 名称。

| 业务位置 | 请求业务字段 | 模型输出字段 |
| --- | --- | --- |
| [媒体改编计划](../crates/host/a4n-orchestrator/src/media_authoring.rs#L190) | user JSON 为 `{target,instruction,source,chapter:{number,title,content},video_reference,previous_plan}`；`source=inputs.sources`，`previous_plan` 为上一版文档或 null。正文及来源引用经 `SourceMarkerMap` 编码，`inputs.dependencies` 不整体透传。[组装](../crates/host/a4n-orchestrator/src/media_authoring.rs#L241) | `AdaptationPlanV1 {schema_version,target,intention,decisions[],units[]}`。`decisions[]={description,reason,source_span_ids[]}`；unit `{id,origin,title,location?,participants[],dramatic_question,objective,obstacle,entry_state,exit_state,source_span_ids[],beats[]}`；beat `{id,kind,is_climax,origin,importance,action,reaction?,dialogue_intent?,entry_state,exit_state,source_span_ids[]}`。 |
| [Galgame 源同步剧本改编](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/regeneration/stages/script.rs#L24) | user JSON 为 `{task,output_contract,input}`；`input={chapters,previous_steps:{adaptation_projection,branch_design}}`，章节正文带来源标记，projection 为实际值。 | `GalgameScriptAdaptation`：`scene_lines[] {scene_index,lines[]}`、`branch_scene_lines[] {branch_id,lines[]}`；line `{index,origin,beat_ref:{source_chapter_id,unit_id,beat_id},line_type,speaker_name?,text,source_span_ids[]}`。 |
| [Galgame 源同步角色演出](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/regeneration/stages/staging.rs#L24) | 同上封套，`input={previous_step:{script_adaptation},speaker_profiles}`。 | 与剧本改编相同包装；line 增加 `speaker_profile_id?`、`presentation`。`presentation={schema_version,visible_cast[],enter[],exit[],focus_character_profile_id\|null,hold,player_knowledge[]}`。 |
| [Galgame 源同步视觉规划](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/regeneration/stages/visual.rs#L25) | 同上封套，`input={previous_steps:{adaptation_projection,character_staging}}`；projection 为实际值。 | `GalgameVisualPlan {backgrounds[],event_cgs[]}`；background `{scene_index,background_note,scene_profile_id?,source_span_ids[]}`；event CG `{scene_index,line_index,cg_note,importance,source_span_ids[]}`。 |

## 2.3 Galgame pipeline：4 个独立入口

四处共用 `GalgameLlmStageInput {project_id,root_project_id,workflow_run_id,stage,checkpoint_key,prompt_name,user_prompt,request_user_prompt,shared_adaptation_projection,source_markers,restore_from_checkpoint,restore_required}`，另有 `prompt_renderer`；温度 `0.25`，JSON mode。实际消息依次为：system `galgame_pipeline_session`、system JSON `{shared_context:{adaptation_projection}}`、user `[当前阶段指令]` 加阶段模板及 `request_user_prompt`。`user_prompt` 还用于输入指纹，不能直接当作最终 user 消息。[消息组装](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/llm/stage/preparation.rs#L63)

阶段 user JSON 统一为 `{task:{prompt_name,instruction},output_contract,input}`；来源正文和 span 引用先编码为来源标记。下表中的 projection 在 pipeline user 内写成 `{"$ref":"system.shared_context.adaptation_projection"}`，其内容已在第二条 system 中提供；2.2 的源同步调用则直接在 user 内放实际 projection。[阶段输入 builder](../crates/host/a4n-orchestrator/src/galgame/pipeline/prompt/messages.rs#L1)

| 阶段 | `input` 字段 |
| --- | --- |
| 分支设计 | `{previous_step:{adaptation_projection}}`。 |
| 剧本改编 | `{chapters,previous_steps:{adaptation_projection,branch_design}}`；`chapters` 含带标记正文。 |
| 角色演出 | `{previous_step:{script_adaptation},speaker_profiles}`。 |
| 视觉规划 | `{previous_steps:{adaptation_projection,character_staging}}`。 |

| 位置 | prompt_name | 输出字段 |
| --- | --- | --- |
| [分支设计](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/early_stages/branch.rs#L96) | `galgame_branch_design` | `{choices:[{after_beat_ref:{source_chapter_id,unit_id,beat_id},question,options:[{branch_id,option_text,branch_title,branch_objective,dramatic_action,reaction,player_knowledge[],returns_to:{source_chapter_id,unit_id}}]}]}`。 |
| [剧本改编](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/early_stages/script.rs#L49) | `galgame_script_adaptation` | `GalgameScriptAdaptation`，字段同 2.2。 |
| [角色演出](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/early_stages/staging.rs#L42) | `galgame_character_staging` | `GalgameCharacterStaging`，字段同 2.2。 |
| [视觉规划](../crates/host/a4n-orchestrator/src/galgame/pipeline/run/pipeline/late_stages/visual.rs#L42) | `galgame_visual_planning` | `GalgameVisualPlan`，字段同 2.2。 |

## 2.4 Manga pipeline：6 个 prompt 调用点

共同输入 `MangaRenderTraceInput {prompt_renderer,prompt_name,project_id,workflow_run_id,call_key,call_order,restore_from_checkpoint,variables,user_content:LlmContextManifest,temperature,cancel_token?,stream_id,stream_reporter?}`。共同 trace 输出 `{prompt_name,request_json,messages_json,response_text,workflow_run_id,call_key,call_order,input_fingerprint_json,usage?,finish_reason?}`。

system 来自对应 `prompt_name` 模板；user 由 manifest 渲染。builder 将输出约束放入 `manga_output_contract`，把下表 `input` 的各顶层字段分别放入 `manga_input_<字段名>` 包，均为 `required=true,stable_prefix=true`，只有 `full_raw` 变体。例外是 `input.current_page` 保留为动态输入，`output_contract.page_number` 保留为动态约束；动态任务形状为 `{task:{prompt_name,instruction},output_contract,input,context_refs}`。[公共组装](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/common.rs#L1)

| prompt 阶段 | 实际业务 `input` |
| --- | --- |
| 改编前预处理 | `{chapter:{content},reference_assets}`；`content` 是带来源标记的完整正文。一次输出事件、角色、对白/旁白、视觉锚点、物品和章节综合。 |
| 页面槽位规划 | `{extraction_summary:{source_events,characters,speech,visual_anchors,items,comic_synthesis},adaptation_plan,source}`；`source` 是带标记正文。 |
| 页面改编 | `{page_number,total_pages,page_content,adaptation_beats,previous_page_content,next_page_content}`。 |
| 页面分镜 | `{page_number,total_pages,page_content,page_adaptation,adaptation_beats,neighboring_pages:{previous_content,next_content},characters,reference_assets:{character_portraits}}`；立绘资料按本次 adaptation 的参与者筛选。 |
| 单页参考图发现 | `{page_number,page_content,page_adaptation,current_page,characters,available_assets}`；只从当前可用图片描述中选择系统参考图，可以返回空列表。 |
| 页面绘图提示词 | `{current_page,page_context:{characters},reference_assets,reference_images,page_content,page_adaptation}`；人物及素材按本页筛选，`current_page` 是动态输入；`reference_images[].source` 区分系统发现与用户补充。 |

上述字段分别由 [预处理](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/preprocessing.rs#L9) 和 [页面阶段](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/messages/page_pipeline.rs#L18) 构造。页面分镜与绘图提示词中的角色/图片资料是文本描述，不等同于 `vision_images` 图片附件。

| 位置 | prompt_name | 模型输出字段 |
| --- | --- | --- |
| [改编前预处理](../crates/host/a4n-orchestrator/src/manga/pipeline/run/preprocessing.rs#L87) | `manga_preprocessing` | `MangaPreprocessingV3 {schema_version,source_events,characters,speech,visual_anchors,items,comic_synthesis}`。 |
| [页面槽位规划](../crates/host/a4n-orchestrator/src/manga/pipeline/run/page_slots.rs#L32) | `manga_page_slots` | `MangaPageSlotsV1 {schema_version,pages:[{page_number,content:{summary,source_span_ids[],beat_refs[],action,reaction?,dialogue_intent?,entry_state,exit_state}}]}`；`pages.length` 必须等于改编计划的 `page_count`。 |
| [页面改编](../crates/host/a4n-orchestrator/src/manga/pipeline/run/page.rs#L602) | `manga_page_adaptation_plan` | `MangaPageAdaptationV1 {visual_intent,continuity_requirements[],retained_details[],omitted_details[],beat_refs[]}`。 |
| [页面分镜](../crates/host/a4n-orchestrator/src/manga/pipeline/run/page.rs#L714) | `manga_page_storyboard` | `MangaPageV4 {page_number,beat_refs[],role,pacing,importance,reading_flow,page_goal,visual_thesis,entry_state,exit_state,dominant_panel_number,panels[]:MangaPanelV2}`。 |
| [单页参考图发现](../crates/host/a4n-orchestrator/src/manga/pipeline/run/page.rs#L862) | `manga_page_reference_discovery` | `MangaPageReferenceDiscoveryV1 {schema_version,page_number,references:[{asset,target_kind,target_name,description,usage}]}`；asset ID 只能来自候选列表。 |
| [页面绘图提示词](../crates/host/a4n-orchestrator/src/manga/pipeline/run/drawing_prompt/page.rs#L46) | `manga_page_drawing_prompt` | `{schema_version,page_number,image_prompt}`。 |

页面分镜的 `panels[]` 是 `MangaPanelV2 {panel_number,beat_refs[],function,size,shot_scale,camera_angle,participants[],composition,observable_action,reaction?,spatial_relationship,continuity_from_previous?,text_layers[]}`；text layer 为 `{layer_index,kind,speaker,text,placement,avoid_covering[]}`。`dialogue/thought` 的 `speaker` 为稳定角色名，`narration/sfx` 为 JSON null；`placement` 限定为 `top_left/top_right/upper_center/center_left/center_right/bottom_left/bottom_right/lower_center`。

`AdaptationBeatRef={unit_id,beat_id}`。页面 enum 值：`role=opening/setup/rising/climax/falling/resolution/transition`；`pacing=slow/medium/fast/explosive`；`importance=critical/high/normal/low`；`reading_flow=top_to_bottom_left_to_right/z_path/s_path/diagonal`。Panel enum 值分别由 `MangaPanelFunctionV2`、`MangaPanelSizeV2`、`MangaShotScaleV2`、`MangaCameraAngleV2` 限定；text layer `kind=dialogue/thought/narration/sfx`。

所有 Manga 输出均为严格 JSON；各阶段 validator 会检查已声明字段、来源 span/beat 引用及数量约束。

## 2.5 拍摄剧本：3 个调用点

共同 stream 入参 `ChatJsonStreamInput {llm,request,prompt_name,truncated_message,structured_progress?,cancel_token?,cancelled_message}`；raw 结果为 `{response_content,restored,restored_parsed_output?,response_only_checkpoint,usage?,finish_reason?,upgraded_contract}`。

system 由公共导演约束和阶段模板组成，温度 `0.2`，JSON mode。每次请求的公共 manifest 包都是 required/stable/full_raw：`shooting_chapter_source={chapter_number,title,summary,directing_statement}`；`shooting_source_spans={content}` 只含本单元来源 ID 覆盖的带标记正文；`shooting_scene_profiles={scene_profiles}` 只保留当前 `scene_binding` 入/出场景。动态文本指定本次单元或镜头。[公共上下文](../crates/host/a4n-orchestrator/src/shooting_script/context.rs#L1)

| 阶段 | 附加上下文包 |
| --- | --- |
| 拍摄设计镜头 | `shooting_visual_design_input={screenplay_plan_unit,locked_visual_unit,target_shot,required_performance_facts}`；`screenplay_plan_unit` 移除重复 `performance_facts`，每次只设计目标镜头，随后合并为场景。[组装](../crates/host/a4n-orchestrator/src/shooting_script/generation/design.rs#L140) |
| 表演事实改编 | `shooting_screenplay_layout_unit={unit,locked_visual_unit,allowed_source_span_ids,allowed_user_addition_ids}`；导演新增事实引用完整的 `director:<segment_id>`，只处理目标 unit。[组装](../crates/host/a4n-orchestrator/src/shooting_script/generation/screenplay/adaptation.rs#L82) |
| 剧本单元布局 | `shooting_layout_skeleton` 含 skeleton（各 unit 移除重复 `source_span_ids`），另加 `locked_visual_unit,previous_unit_end,next_unit_start`；相邻单元只取末/首镜头，动态指令限定 `narrative_unit_index`。[组装](../crates/host/a4n-orchestrator/src/shooting_script/generation/screenplay/layout.rs#L169) |

| 位置 | 返回类型 | 输出字段 |
| --- | --- | --- |
| [拍摄设计镜头](../crates/host/a4n-orchestrator/src/shooting_script/generation/design.rs#L75) | `ShootingSceneV2` | `scene_index,scene_identity,sequence_index,narrative_unit_index,timeline_position,intercut_group_id?,editing_mode,transition_contracts[],title,slugline,location?,time_of_day?,participants[],focal_character?,source_span_ids[],visual_narrative_unit_id?,user_addition_ids[],dramatic_question,scene_objective,scene_obstacle,relationship_before,relationship_after,audience_knowledge_before[],audience_knowledge_after[],withheld_information[],viewpoint,spatial_continuity,sound_strategy,entry_state,exit_state,shots[]`。每个 shot 含 `shot_index,shot_id,title,source_span_ids[],visual_shot_id?,planned_start_ms?,planned_end_ms?,user_addition_ids[],visual_constraints?,coverage,narrative_purpose,subject,shot_size,camera_angle,camera_height,lens_intent,camera_movement,composition,spatial_anchors[],blocking,performance_beats[],dialogue_cues[],sound,camera_start_state,camera_end_state,start_state,end_state,transition_in,transition_out,continuity_requirements[]`。 |
| [表演事实改编](../crates/host/a4n-orchestrator/src/shooting_script/generation/screenplay/adaptation.rs#L38) | `ScreenplayUnitAdaptationV1` | `schema_version,narrative_unit_index,performance_facts[]`；fact 为 `fact_index,shot_id,source_span_ids[],user_addition_ids[],actor?,objective,obstacle,visible_action,reaction?,dialogue_intent?,entry_state,exit_state,required`。 |
| [剧本单元布局](../crates/host/a4n-orchestrator/src/shooting_script/generation/screenplay/layout.rs#L121) | `ScreenplayLayoutUnitDetailV2` | `schema_version,narrative_unit_index,source_span_ids[],transition_contracts[],title,slugline,participants[],dramatic_question,scene_objective,scene_obstacle,entry_state,exit_state`。 |

## 2.6 大纲和立绘：5 个调用点

这五处是旧清单确实漏掉的入口：

| 位置 | 上游业务输入 | 模型输出 |
| --- | --- | --- |
| [分部大纲](../crates/host/a4n-orchestrator/src/outline/parts/llm/execution.rs#L86) | `PartOutlineJsonInput {project_id,part_num,start_ch,end_ch,chapters_per_part,requested_part_count,blueprint_context,previous_parts,source_snapshot,prompt_renderer,variables,workflow_run_id?}`。 | 严格 JSON：`title,summary,theme,key_events[],character_focus`；至少 3 个 `key_events`。系统补分部号和起止章。 |
| [节拍映射生成章节大纲](../crates/host/a4n-orchestrator/src/outline/chapters/beat_mapping/execution.rs#L72) | `BeatMappingApplyInput {project_id,part_id,beat_sheet,prompt_renderer,variables,progress?,workflow_run_id?}`，服务补入分部、节拍和来源上下文。 | `{chapters:[...]}`；每项字段同章节大纲 schema；数量必须匹配目标章节数，`chapter_number` 系统补入。 |
| [单章大纲](../crates/host/a4n-orchestrator/src/outline/chapters/llm/execution.rs#L72) | `SingleChapterOutlineJsonInput {project_id,context,focus_part?,chapter_number,prompt_renderer,variables,workflow_run_id?}`。 | `{chapters:[one item]}`；字段同上，只允许一项。 |
| [人物初始三视图提示词](../crates/host/a4n-orchestrator/src/portrait/application/workflow.rs#L97) | `PortraitPromptSpec {profile_name,appearance?,gender?,age?,personality?,background?,profile_context?,project_context?,portrait_type,art_style?,expression?,existing_portrait_count?}`；温度 `0.8`。 | 纯文本提示词；拒绝空文本、JSON、数组、Markdown 标题/fence 和 `<think>`。 |
| [单次立绘提示词生成](../crates/host/a4n-orchestrator/src/portrait.rs#L250) | 同一 `PortraitPromptGenerationInput`/`PortraitPromptSpec`；payload 合并 profile、项目上下文、立绘类型及可选风格/表情。 | 同为纯文本 prompt；checkpoint 保存 `{text,usage?,finish_reason?}`。 |

模型消息中的实际内容如下；这些上下文包均为 required/stable/full_raw，`source_snapshot` 参与输入指纹，不整体作为消息发送。

| 调用 | system / 稳定上下文包 | 动态 user 内容 |
| --- | --- | --- |
| 分部大纲 | system `part_outline_single`；`outline_blueprint_context={kind,blueprint}`；有前序分部时加 `outline_previous_part_outlines={kind,parts[]}`。温度 `0.7`，JSON mode。[组装](../crates/host/a4n-orchestrator/src/outline/parts/requests/request.rs#L13) | 任务/输出要求及 `{context_refs,generation_request:{chapters_per_part,requested_part_count,part_number,start_chapter,end_chapter},previous_parts:[{part_number,context_ref}]}`。 |
| 单章大纲 | system `outline`；`outline_blueprint_context` 及 `outline_part_outlines={kind,parts[]}`。温度 `0.7`，JSON mode。[组装](../crates/host/a4n-orchestrator/src/outline/chapters/requests/request.rs#L13) | 任务/输出要求及 `{context_refs:{novel_blueprint,part_outlines,active_part_outline},wait_to_generate:{start_chapter,num_chapters,end_chapter}}`；单章调用的 `num_chapters=1`。 |
| 节拍映射 | 复用上述章节大纲两包与 `outline` system，温度 `0.6`。[组装](../crates/host/a4n-orchestrator/src/outline/chapters/beat_mapping/request.rs#L14) | 与章节大纲相同的引用和目标范围，另加 `beat_sheet`（非空 JSON 对象）；按目标分部生成整批章节。 |
| 两个立绘调用 | system `character_portrait_generation`；`portrait_profile_project_context={profile_name,appearance,gender,age,personality,background,profile_context,project_context}`。温度 `0.8`，纯文本。[组装](../crates/host/a4n-orchestrator/src/portrait/prompt_generation/request.rs#L21) | `character_portrait_generation_user` 渲染 payload：`context_refs,character_name,portrait_type,profile_context,project_context,art_style,expression,existing_portrait_count`，其中两项 context 为包内引用或 null；另有三视图约束 `image_goal,required_views,breakdown_sections,layout_spec,detail_focus,composition_constraints,detail_policy,negative_constraints`。 |

大纲 `blueprint` 是面向模型的投影，含已有蓝图字段或项目基础信息，另有 `blueprint_available,chapters_per_part,blueprint_chapters_per_part,characters,relationships,formal_materials`；`formal_materials` 包含 `character_profiles,objects,scenes,foreshadowing`，伏笔排除 resolved/abandoned。角色与正式档案相同的外貌等字段会去重，`total_chapters` 被移除。[来源投影](../crates/host/a4n-orchestrator/src/outline/context/projections.rs#L14) `parts[]` 仅含 `part_number,title,start_chapter,end_chapter,summary,theme,key_events,character_focus`。

章节大纲响应 `chapters[]` 必含 `title,summary`，可选且只可增加 `beats,key_scenes,pacing_score,foreshadowing_setup,foreshadowing_payoff,characters_involved,word_count_target`；`chapter_number` 由系统按数组顺序补齐。[输出合同](../crates/host/a4n-orchestrator/src/outline/chapter_contract/prompt.rs#L37)

## 2.7 导入反推 target 输出字段

主表 #9 的 `target` 有三个固定合同：`blueprint` 必须有 `title,genre,style,tone,target_audience,one_sentence_summary,full_synopsis,world_setting,characters[],relationships[]`，可选 `total_chapters,needs_part_outlines`；人物项必须有 `name,character_type,gender,age,appearance,personality,background,motivation,arc_summary,relationship_to_protagonist`，`arc_stages?` 可选；关系项必须有 `character_a,character_b,relationship_type,description`，`strength?`、`evolution?` 可选。`world_setting` 字段见 2.8。`outline` 为 `{outlines:[{chapter_number,title,summary,word_count_target?}]}`；`part_outline` 为 `{parts:[{part_number,title,start_chapter,end_chapter,summary,theme,key_events[],character_focus}]}`。候选封套为 `{workflow_run_id,target,state,warnings[],result,applied?}`。

## 2.8 蓝图生成阶段字段

主表 #4 的 transport 在一个四阶段循环中调用。`BlueprintStageSpec` 的输出分别是：

| stage | 模型 JSON |
| --- | --- |
| `story_core` | `title,target_audience,genre,style,tone,one_sentence_summary,full_synopsis`。 |
| `world` | `world_setting:{premise,core_rules,technology_or_magic:{source,rules[],limits[],costs[]},key_locations[]:{name,description,story_function},factions[]:{name,description,agenda,representative_characters[]},history_timeline[]:{period,event,impact},culture_and_society:{values,class_structure,daily_life},conflict_sources[]:{name,description,stakes},mysteries_and_foreshadowing[]:{mystery,early_clues[],payoff_direction},genre_promises[]}`。 |
| `characters` | `characters[]:{name,character_type,gender,age,appearance,personality,background,motivation,arc_summary,relationship_to_protagonist,arc_stages:{start,core_wound,desire,need,secret,voice_style,turning_point,end}}`。 |
| `relationships` | `relationships[]:{relationship_type,character_a,character_b,description,strength,evolution:{start,tension,turning_points[],hidden_truth,story_function,end}}`。 |

## 2.9 章节分析四阶段字段

主表 #22 的四种 stage request 共用同一个结构化流入口；root keys 均做 unknown-field 检查：

| 阶段 | JSON 字段 |
| --- | --- |
| facts | `characters[]`、`locations[]`、`items[]`（实体项 `{name,source_span_ids[]}`）；`event_candidates[]:{event_ref,type,description,importance,involved_characters[],source_span_ids[]}`；`foreshadowing_candidates[]:{foreshadowing_ref,description,category,priority,related_entities[],original_text_span_id}`；`world_facts[]:{field,subject,statement,assertion,source_span_ids[]}`。`field` 为 `premise/core_rules/technology_or_magic/key_locations/factions/history_timeline/culture_and_society/conflict_sources/mysteries_and_foreshadowing/genre_promises`；`assertion` 为 `observed/inferred/reported/hypothetical/recollection`。`evidence` 系统派生，模型不得输出。 |
| summary | `{metadata_additions:{tags[],tone?,timeline_marker?},summaries:{compressed,one_line,keywords[]}}`。 |
| character state | `{character_states:{角色名:{location?,status?,changes[],emotional_state?}}}`。 |
| foreshadowing/events | `{foreshadowing:{planted[],resolved[],progressed?,tensions?},key_events[]}`。`planted[]` 可含 `id?`，必含 `description,original_text_span_id,category,priority,related_entities[]`；系统从来源标记补 `original_text`。`resolved[]` 使用 `id,resolution,source_span_ids[]`；`progressed[]` 另需 `status`，其值为 `pending/partially_revealed/resolved/abandoned`。二者都引用既有伏笔 ID。`key_events[]` 为 `{type,description,importance,involved_characters[],source_span_ids[]}`，`type=battle/revelation/relationship/discovery/decision/death/arrival/departure`，`importance=high/medium/low`。 `tensions` 出现在 prompt 示例中，但当前 validator 不固定其元素字段。 |

## 2.10 视频提示词按 prompt 区分的输入/输出

主表 #25–#26 共用请求组装：system 为 `video_prompt_session` 加当前阶段模板，user 为 manifest，JSON mode；温度由具体调用传入。[请求组装](../crates/host/a4n-orchestrator/src/video/prompts/generation/llm_chat/request.rs#L18) 常规阶段的稳定包 `video_chapter_source={content}` 只放来源正文，生成器按当前镜头/片段覆盖的 source spans 选取摘录；摘要和分析不会在此包内重复发送。非空的前序输出包及 `video_project_context` 按调用添加，加入后也是 required/stable/full_raw，不能因 helper 名为 optional 就当成预算可丢弃包。[manifest](../crates/host/a4n-orchestrator/src/video/prompts/context/ledger/manifest.rs#L11)

动态 user 通过 `chapter_video_user_request` 渲染任务指令和下表 payload。常规帧/视频 payload 的 `chapter={summary,analysis}` 中，analysis 只投影 `compressed_summary,key_events,metadata`（存在时）；不是全部章节分析。

| prompt family | 动态业务输入（另含 `task,constraints,output_contract`） |
| --- | --- |
| `chapter_video_segment_plan` | `chapter:{summary},current_shot`；当前镜头及其 `action_beats` 删除重复 `source_span_ids`。constraints 指定风格、固定时长、画幅及来源/节拍/镜头映射规则；每次仅规划当前镜头的 1 个 segment。[builder](../crates/host/a4n-orchestrator/src/video/prompts/context/builders/planning.rs#L10) |
| `chapter_video_frame_prompts` | `chapter,previous_steps:{focused_chapter_video_segment_plan},current_frame,available_assets`；约束含风格、语言、画幅、参考图槽位和相邻帧连续性，输出限定 1 帧。[builder](../crates/host/a4n-orchestrator/src/video/prompts/context/builders/frame_prompts.rs#L14) |
| `chapter_video_first_frame_prompts` | `chapter,previous_steps:{chapter_video_segment_plan:{segments:[首片段]}},available_assets`；只用首片段生成初始帧，附语言/风格/画幅/参考图槽位约束。 |
| `chapter_video_storyboard_prompts` | 与普通帧相同，但当前目标为 `current_storyboard_frame`；输出为当前视频单元的 1 张多格分镜板。 |
| `chapter_video_prompt`、`chapter_video_first_frame_video_prompt`、`chapter_video_chat_prompt` | `chapter,previous_steps:{focused_chapter_video_segment_plan,previous_completed_shot},current_segment,reference_asset_summaries,input_frames`；约束包含风格、语言、时长、画幅和各模式的图片使用/连续性规则。[首尾帧 builder](../crates/host/a4n-orchestrator/src/video/prompts/context/builders/boundary_video.rs#L14) |
| `chapter_video_storyboard_video_prompt` | 与上述最终视频输入相同，但用 `storyboard_frame` 替代 `input_frames`，增加分镜板使用约束。[builder](../crates/host/a4n-orchestrator/src/video/prompts/context/builders/storyboard_video.rs#L13) |
| `chapter_video_inserted_segment` | 单独构造无 pack 的动态 manifest：`chapter:{project_id,chapter_id,summary,content_sha256,content_excerpt},timeline:{timeline_id,expected_revision,previous_item_id,next_item_id},previous_item,next_item,user_prompt,style,prompt_output_language,prompt_output_language_instruction,visual_prompt_required_sections,motion_prompt_required_sections,visual_prompt_min_chars,motion_prompt_min_chars,constraints`；constraints 为 `{selected_duration_secs,duration_secs_options,aspect_ratio,guidance_mode,continuity_source_video_id,reference_image_ids}`。温度 `0.45`。[组装](../crates/host/a4n-orchestrator/src/video/prompts/generation/frame_prompts/inserted_segment.rs#L191) |

最终视频的四种 builder 还会递归移除内部来源 span、素材/图片/项目/profile ID、reference binding 和展示标签等键，并将 `dialogue_beats` 改名、投影为 `dialogue_cues`。[公开字段投影](../crates/host/a4n-orchestrator/src/video/prompts/context/public_context.rs#L23) 该规则不适用于上表单独构造的插入片段 payload，后者明确包含 ID。输入中的素材/帧信息是文本 JSON 资料；这些 prompt builder 没有因此自动添加 `vision_images`。

`response_text` 由 `prompt_name` 对应 parser 解析：

| prompt family | 模型 JSON |
| --- | --- |
| `chapter_video_frame_prompts`、`chapter_video_first_frame_prompts` | `{frames:[{title,scene_summary,frame_prompt,blocking_state,lighting_state,negative_prompt}]}`。 |
| `chapter_video_storyboard_prompts` | 同上，另有 `storyboard_panels[]:{panel_index,moment,visual_goal,composition,continuity_note}`。 |
| `chapter_video_prompt`、`chapter_video_first_frame_video_prompt`、`chapter_video_storyboard_video_prompt` | `{shots:[{title,scene_summary,visual_prompt,motion_prompt,negative_prompt,dialogue_cues}]}`。 |
| `chapter_video_chat_prompt` | `{shots:[{title,scene_summary,chat_prompt,negative_prompt}]}`。 |
| `chapter_video_segment_plan` | `{segments:[{source_shot_id,covered_beat_ids[],title,scene_summary,blocking_summary,spatial_anchors[],emotion_action_goals[],video_intent,emotional_arc,performance_plan,material_coverage}]}`。`spatial_anchors[]:{anchor,usage}`；`emotion_action_goals[]:{character,emotion,visible_action}`；`video_intent:{hook,audience_effect,rhythm,visual_priority}`；`emotional_arc:{start,turn,end,function}`；performance plan 含 `movement_path,body_performance,gaze_and_hands,pause_or_reaction,camera_relationship,time_blocks[],action_script[]`；coverage 含 `required_entities[],missing_or_weak_references[],risk_note`。 |
| `chapter_video_inserted_segment` | `{title,scene_summary,visual_prompt,motion_prompt,negative_prompt}`。 |

shot/frame number、时长、画幅和引用素材 ID 由已知片段或默认值派生，不要求模型返回。

帧提示词中的 `blocking_state`、`lighting_state` 和视频提示词中的 `dialogue_cues` 是 `serde_json::Value` 子树；当前合同固定外层字段，内层键没有统一的 Rust schema。

## 2.11 Chat Completions 媒体生成接口（不计入 49 个聊天 LLM 入口）

两条 provider 也调用 `/chat/completions`，但输入/输出是媒体生成合同：

| provider | 输入/线上字段 | 响应消费 |
| --- | --- | --- |
| [ChatCompletionsImageProvider](../crates/image/a4n-image-providers/src/chat.rs#L41) | `ImageGenerationRequest {prompt,width,height,model,reference_images[]}`；参考图为 `{label,data_uri}`。wire body 含 `model,stream:false,modalities:[image,text],n:1,size,messages[]`；有参考图时 content 为 `{type:text,text}` 和 `{type:image_url,image_url:{url:data_uri}}` parts。 | 从 `choices[0].message.images/content` 或文本中提取 URL/base64；结果 `{bytes,format,model}`。 |
| [ChatCompletionsVideoProvider](../crates/video/a4n-video-providers/src/chat.rs#L70) | `VideoGenerationRequest {prompt,negative_prompt?,width,height,duration_secs,fps,model,media}`；wire body 含 `modalities:[video,text],n,size,duration_secs,fps,negative_prompt,reference_image_urls,image?,last_frame?,messages[]`。 | 提取 `choices[0].message.video/videos/content` 中的视频 URL/base64；结果 `{bytes,format,provider_job_id:null,metadata}`。 |

## 2.12 其余调用的模型可见业务输入

本表补充主表中只列 DTO/执行器参数的调用。表内 JSON 是消息中的业务 payload；任务说明、模板文本和独立输出契约仍按各 builder 渲染，不是同名 HTTP 顶层字段。manifest 包须经过 1.4 的选取；`variables` 是模板渲染参数，只有模板引用的内容才成为 system/user 文本。

| 主表 # / 业务 | 上下文包或实际消息来源 | 动态字段与输入范围 |
| --- | --- | --- |
| 1 章节正文 | system `writing`；`writing_chapter_outline={kind,chapter_outline}`；前文非空时加 `writing_context_before={kind,context_before}`，均 required/stable/full_raw。[请求](../crates/prompt/a4n-prompt/src/chapter_generation/request.rs#L19) | 动态 `{task:{name,instruction},context_refs:{chapter_outline,context_before},output_contract}`。`chapter_outline` 已拼入章纲/写作指令/字数范围、关键参考和本次检索的 RAG 文本；`context_before` 的组成见表后。 |
| 2 蓝图优化 | `inspiration_conversation_context={kind,conversation:{messages:[{role,content}]}}`；`existing_blueprint_context={kind,blueprint_context:{blueprint,characters,relationships}}`。[上下文](../crates/prompt/a4n-prompt/src/blueprint_generation/refine/context.rs#L11) | user 模板 payload 为 `{context_refs,focus?,output_contract}`；灵感历史先于现有蓝图，二者都是 required/stable/full_raw。 |
| 3 局部补丁 | `optimization_patch_chapter_content={kind,content}`；`optimization_patch_project_context={kind,context}`，context 来自分析上下文。[请求](../crates/host/a4n-orchestrator/src/optimize/patch/request.rs#L1) | `{task:{name,requirement},context_refs,chapter:{base_content_hash},selected_issues}`。项目上下文移除重复 `current_chapter.content` 和 `project_detail.export_config`；有独立 blueprint 时还移除 `raw_blueprint`。system 模板另渲染权限、能力及 Agent 指令变量。 |
| 4 蓝图阶段生成 | `blueprint_project_seed_context={kind,source_input}`；有前序结果时加 `blueprint_previous_stage_outputs={kind,previous_context}`。[上下文](../crates/prompt/a4n-prompt/src/blueprint_generation/stage_request/context.rs#L11) | source_input 必含 `novel_title,genre,requested_chapter_count`，仅在传入时增加 `inspiration_transcript,generation_instruction,conversation_state,progress_summary,next_question,project`；动态 `{context_refs,current_stage:{key,name,output_schema}}`。previous_context 为已完成阶段输出。 |
| 5 优化 Agent 决策 | system 优化 Agent 规则；`optimization_agent_rules={completion_policy}`、`optimization_agent_base_task={kind,base_task}` 及独立输出契约。[消息组装](../crates/host/a4n-orchestrator/src/optimize/agent_loop/request/envelope.rs#L1) | 每个 observation 作为单独 user 消息 `{loop_observation:{sequence,value}}`；最后 user 为 `{context_refs:{base_task},loop_state:{step,progress:{observation_count},observations_ref}}`。此处 observation 不是原生 tool 消息。 |
| 6 章节视觉规划 | 稳定 `director_scene_context={chapter_title,chapter_summary,scenes}`；可变 `director_segment_prompt={duration_ms,existing_prompt,scene_binding,keyframes:[{relative_at_ms,label,description}],explicit_end_frame,user_description}`。[组装](../crates/host/a4n-orchestrator/src/chapter_visual_plan/generation.rs#L194) | 动态指令为生成当前镜头段的变化描述。`existing_prompt` 与新描述相同时置 null；`explicit_end_frame` 为 `{relative_at_ms,description}` 或 null。record/revision/计划 hash 属于校验和 trace，不整体加入消息。 |
| 7 灵感对话 | system `inspiration`；`inspiration_project_context={guard,project_context}`。project_context 含 `project`，可选 `blueprint,recent_chapters`；recent_chapters 只投影 `chapter_number,title,summary,word_count`。[请求](../crates/host/a4n-orchestrator/src/inspiration/request.rs#L1) | 首条 user 提供 manifest 任务和输出契约，随后追加历史 role/content 和本轮 user_input；不是只发送用户当前一句话。 |
| 9 导入反推 | system 取 target 对应模板；`source_artifact` 包来自 content_sample：可解析为 JSON 时为 `{source_context:解析值}`，否则为 `{source_text:样本文本}`。[来源](../crates/host/a4n-orchestrator/src/import_reverse/request/payload/source_artifact.rs#L6) | 动态 `{context_refs:{source},task:{prompt_name,instruction},output_contract}`；反推范围由实际样本限定。 |
| 10 视频提示词优化 | 两个优化模板合并为 system，并渲染语言/资产输出契约；稳定 `source_artifact={artifact_kind,current_prompt:{prompt_type,workflow_mode,shot_number,title,scene_summary,duration_secs,aspect_ratio,visual_prompt,motion_prompt,negative_prompt}}`。[组装](../crates/host/a4n-orchestrator/src/video/prompts/optimization.rs#L452) | 动态仅 `{instruction}`；原记录的 metadata、参考图 ID、下游删除选项用于业务处理，不整体传入这个上下文包。 |
| 11 场景正文 | `chapter_scene_generation_context={chapter:{chapter_number,title,summary},available_context}`；available_context 按实际存在情况包含 `chapter_content,chapter_outline,existing_scenes,existing_target_scene`。[上下文](../crates/host/a4n-orchestrator/src/chapter_scene/context.rs#L41) | `{task,input:{context_refs,scene:{scene_number,prompt}},output_contract}`；existing_scenes 排除目标场景，目标旧内容单独保留。 |
| 12 角色冲突检查 | `character_conflict_source_context={chapter_scope,character_profile,chapter_context,character_state_timeline,attribute_events}`。[请求](../crates/host/a4n-orchestrator/src/character/conflict/request.rs#L31) | `{task,input:{context_refs},output_contract}`；输出 expected/actual 的 source_id 只能选输入中已有 ID。 |
| 13 角色档案同步 | system `character_sync_session`；`profile_sync_chapters={chapters:[{chapter_number,content}]}`（正文含 `⟦id⟧`）；`profile_sync_source_context={character:{profile_name},sync_range:{start_chapter,target_chapter,input_chapter_numbers},baseline_state}`。[请求](../crates/host/a4n-orchestrator/src/character/profile_sync/stages/request.rs#L1) | user 前置阶段模板，payload 为 `{context_refs,task:{name,instruction},output_contract,previous_steps}`。events 无前序依赖；attribute_changes/behaviors 只接收 events；chapter_states/deletion_candidates 接收 events 和 attribute_changes，不重复发送全部前序结果。 |
| 14 隐性属性复审 | `implicit_update_review_context={profile_name,attribute_key,current_value,review_window:{window_chapters,non_conform_threshold},statistics:{total,conform_count,non_conform_count,conform_rate,threshold_reached},evidence[]}`。[请求](../crates/host/a4n-orchestrator/src/character/implicit_review/request.rs#L1) | evidence 项含 `behavior_id,chapter_number,behavior_type,behavior_description,original_text,behavior_tags,classification`；动态含任务、角色名、目标属性 key、context_refs 和输出契约。 |
| 15 隐性属性分类 | `implicit_classification_context={current_implicit_attributes}`。[请求](../crates/host/a4n-orchestrator/src/character/implicit_classification/request.rs#L31) | `{task,character:{profile_name},input:{context_refs,behavior:{chapter_number,behavior_type,behavior_description,behavior_tags,original_text}},hard_rules,output_contract}`。 |
| 17 漫画提示词优化 | system `manga_prompt_optimization`；普通 user 模板 `manga_prompt_optimization_user`，没有该业务的 manifest 包。[请求](../crates/host/a4n-orchestrator/src/manga/prompt_optimization/request.rs#L18) | payload 为 `{current_prompt:{prompt_id,page_number,dialogue_text,image_prompt},instruction,output_contract}`。 |
| 20 角色发现 | system `character_discovery`；`profile_sync_source_context={existing_profiles,chapters:[{chapter_number,content}]}`；content 为本次所选章节的 `⟦id⟧` 标记正文。[请求](../crates/host/a4n-orchestrator/src/character/discovery/request.rs#L1) | `{task,input:{context_refs},output_contract}`；章节选择先由服务完成，DTO 的 project/chapter 参数不是整份 prompt。 |
| 21 场景档案同步 | system `scene_profile_sync_session`；`scene_profile_sync_source_context={existing_scene_profiles,available_characters,available_object_profiles,chapter}`，chapter 正文含来源锚点。[请求](../crates/host/a4n-orchestrator/src/scene_profile/application/stages/request.rs#L51) | 阶段指令及 `{task,output_contract,input:{context_refs,previous_stages}}`；当前 candidates 调用传 `previous_stages={}`，只识别或匹配 scenes。 |
| 22 章节分析 | system `chapter_analysis_session`；公共 `chapter_analysis_source={chapter_number,chapter_title,content}`，content 为带 `#序号` 来源标记的本章正文。[请求](../crates/host/a4n-orchestrator/src/chapter_analysis/owner/requests/context.rs#L23) | 阶段 user 含 `{task,input,output_contract}`；input 删除重复 content，改加 `chapter_source_context` 引用。facts 传 chapter_title；summary 另传 facts；character_state 另传 characters/event_candidates；foreshadowing 另传 `facts:{event_candidates,foreshadowing_candidates}` 和 pending_foreshadowing。 |
| 23 物体发现 | `profile_sync_source_context={existing_object_profiles,chapters:[{chapter_number,content}]}`，content 为 `⟦id⟧` 标记正文。[上下文](../crates/host/a4n-orchestrator/src/object_discovery/context.rs#L9) | `{task,input:{context_refs},output_contract}`；existing_object_profiles 是物体资料投影，包含名称/别名、类别、重要度、外观/材质状态、用途、位置及已见章节范围。 |
| 24 正文事实归纳 | `current_materials={blueprint,characters,part_outlines,planning_rule}`；`body_fact_sources={chapters:[{id,chapter_number,title,content,world_facts}]}`，正文带来源标记。[请求](../crates/host/a4n-orchestrator/src/fact_feedback/request.rs#L94) | 动态指令要求归纳 changes，另有独立输出约束。blueprint 只取 `id,version,full_synopsis,world_setting`；characters 取 `id,name,current_state,voice_fingerprint,character_arc`；part_outlines 取 `part_number,start_chapter,end_chapter,summary,character_focus,key_events`。 |
| 29 图片视觉评估 | system 为视觉检查规则；user 为 instruction 或空输入时的默认说明；实际图片进入 `vision_images`。[组装](../crates/host/a4n-runtime/src/actions/image_records/reading.rs#L86) | 温度 `0.1`、输出上限 2048；project/image/chapter ID 用于查图、权限与 trace，不自动以同名字段加入 user。 |
| 30 画布文本素材草稿 | 固定文本创作 system；user 为指令，可选参考素材正文，最后要求只返回正文。[组装](../crates/host/a4n-runtime/src/actions/canvas_workflow/text_asset_draft.rs#L79) | source_asset_id/source_version 用于读取指定素材版本，消息中加入的是该版本 content；无来源时只包含 instruction。 |
| 31 画布文本生成端口 | 固定文本转换 system；`canvas_upstream_text={text:source_context}`、`canvas_node_config:LlmTextNode={output_kind,instruction}` 两个 required/stable/full_raw 包。[请求](../crates/host/a4n-orchestrator/src/canvas/text_generation/request.rs#L39) | 任务说明加 `{context_refs:{node_config,input_text},output_contract:{format,output_kind,rules}}`；temperature/max_tokens 来自节点配置。source_context 来自连接的上游文本，graph/node/config ID 进入 trace/来源元数据。 |

章节正文 #1 的 `writing` 模板可用变量为 `chapter_number,chapter_title,chapter_outline,chapter_word_count_min,chapter_word_count_max,chapter_word_count_range`，以及非空时的 `writing_instruction,generation_reference_context,generation_rag_context`。[变量](../crates/host/a4n-orchestrator/src/chapter_generation/preparation/variables.rs#L7) 关键参考先按角色、关系、主角档案、待回收伏笔选取，并格式化核心设定和世界观；不是直接塞入全部项目记录。`context_before` 包含适用的已确认正文世界事实、角色状态、物体/场景档案、分层前文摘要、最近 12 章的结构化线索及上一章末尾最多 600 字符。[前文组装](../crates/host/a4n-orchestrator/src/chapter_generation/context_before/resolution.rs#L13)

共享 driver 的其余业务输入见 2.2–2.6，视频见 2.10，Agent 主回合与压缩见 6.10；这些展开说明不额外增加静态调用点数量。

## 3. 配置连通性探测（不计入上述 49 处）

[`test_llm_config`](../crates/host/a4n-runtime/src/actions/config_records/llm/connectivity.rs#L14) 使用 `LlmProviderConfig {id,name,provider,base_url,api_key?,model,max_concurrent,timeout_secs,max_context_tokens,max_output_tokens,extra_body_json?,agent_protocol,pricing}`；probe `ChatRequest` 为单条 user 消息 `"ping"`、`temperature=0.0`、`max_tokens=min(512, model.max_output_tokens)`。Sidecar profile 调 `probe_with_config`；非 sidecar profile 调 `chat_with_config`。sidecar probe 结果为 `response_preview`、`usage?`、`finish_reason?`；上层接口只返回成功 `()`，空响应是错误。

## 4. 流程中的字段保留与裁剪

- `ChatChunk -> LlmStreamChunk` 的 sidecar 映射逐字段保留：文本、reasoning、tool calls、finish reason、index 和 usage；再由 `CapabilityTaskEvent` 加 `task_id/event_kind/sequence` 封装。
- `execute_llm_generation` 聚合 `content`、usage、finish reason 和 `content_sha256`；reasoning 单独发到 `stream_id="reasoning"`，不在 `LlmGenerationOutput` 结构内；这段 driver 没有读取 `tool_calls`。
- `chat_json_stream_result_with_sink` 只聚合 `content`、usage、finish reason；`StructuredLlmStreamResult` 不含 reasoning 或 tool calls。JSON 字段由上表里的调用者 parser/validator 负责。
- `ChatResponse` 类型可容纳 assistant message、message-level `tool_calls`、`reasoning_content`、usage 和 finish reason。Agent 与 Runtime 使用的流聚合器可保留这些字段；业务包装器不能据返回类型推断保留行为。
- `stream_complete_llm*` → `execute_llm_chat_response_with_sink`，以及大纲/立绘使用的 `execute_task_llm_chat_response`，都先调用 generation driver，再用 `ChatMessage::assistant(output.content)` 重建消息；因此返回的 `tool_calls=None`、`tool_call_id=None`、`reasoning_content=None`。usage 被转换回 provider 类型，finish reason 被转换成统一文本。这是包装器已丢弃字段，不只是上层没有读取。[响应组装](../crates/host/a4n-orchestrator/src/generation/stream.rs#L98)
- generation task 对前端发 `TextDeltaEvent {stream_id, offset, delta, chunk_index}` 和 `UsageEvent {usage, cumulative}`；这和 sidecar `CapabilityTaskEvent` 是两套不同封套。

## 5. 读表注意

1.1–1.3 记录应用与 sidecar 类型；模型实际输入还取决于 1.4 的上下文选取、业务 prompt builder 和 1.5 的供应商适配。下文的业务 DTO、执行器参数、trace/cancel/progress 句柄不是自动发送给模型的字段。业务上下文的消息内容应按对应输入表与 builder 阅读，不能只查看某个 `ChatRequest` 类型名。

52 是静态业务调用表达式数，不是 52 种固定 prompt 或 52 次运行时请求。并非所有调用都有静态 Rust 输出 DTO。表中注明 `Value`、`T` 或 prompt-specific JSON 的调用，输出字段集合由对应 parser/validator 或 prompt 名称决定，不存在一个更窄的统一合同。

当前仍有几块内层 JSON 没有固定字段合同：Agent `tools[].function.parameters` 按工具 `args_schema` 变化；视频帧的 `blocking_state`/`lighting_state` 与视频 shot 的 `dialogue_cues` 是 JSON 子树；章节分析的 `foreshadowing.tensions` 有 prompt 示例但无 nested validator。评估这些路径时，应把表中这些字段视为可变 JSON。

## 6. 字段含义

以下按合同解释字段语义。相同底层类型只说明一次；业务字段的含义按所在对象解释。对 `Value` 或任意 JSON 子树，只说明代码明确消费的用途，不把 prompt 暗示扩写成未经验证的合同。

### 6.1 模型请求、消息和响应

| 对象/字段 | 含义 |
| --- | --- |
| `ChatRequest.messages` | 按顺序提供给模型的对话上下文。`system` 写任务规则，`user` 写本次输入，`assistant` 可放历史模型输出，`tool` 用于工具结果回填。 |
| `ChatRequest.vision_images` | 应用层图片附件；`media_type` 是 MIME 类型，`data_base64` 是 Base64 图片内容。适配器将其编码为消息中的图片块，不直接发送同名 HTTP 顶层数组。 |
| `temperature` / `max_tokens` | 采样温度和本次生成 token 上限；它们控制生成，不是 prompt 文本。 |
| `response_format.type` | 要求 provider 使用的响应格式，例如 `json_object`。 |
| `stream_include_usage` | 要求流式响应在结束时附带 token usage。 |
| `cache_scope_key` / `cache_breakpoints` / `cache_mode` | 指定 prompt cache 的隔离范围、消息/字节边界和缓存策略。breakpoint 的 `message_index` 指消息序号，`byte_offset` 指该消息 UTF-8 内容的字节位置。 |
| `tools` / `tool_choice` | 原生工具协议使用的工具定义及选择策略；Agent `json_compat` 将工具 schema 写入 system prompt，顶层二者为空。Agent 协议见 6.10。 |
| `stop_sequences` | 模型输出遇到这些字符串时允许提前停止。 |
| `ChatMessage.role` / `content` | `role` 标明该条消息是谁产生的；`content` 是该角色对应的文本。 |
| `ChatMessage.tool_calls` / `tool_call_id` | assistant 消息发起的工具调用列表；tool 回复里的 `tool_call_id` 将结果关联到对应调用。 |
| `ToolCall.id` / `name` / `arguments` | 本次工具调用的关联 ID、工具名、传给工具的 JSON 参数。 |
| `ToolCall.stream_index` / `provider_metadata` | 流式拼接分段工具调用时的序号；provider 需要保留的扩展元数据。 |
| `UsageInfo.prompt_tokens` / `completion_tokens` / `total_tokens` | 输入 token 数、输出 token 数、二者合计。 |
| `UsageInfo.prompt_cache_hit_tokens` / `prompt_cache_miss_tokens` / `prompt_cache_write_tokens` | 输入中命中缓存、未命中缓存和写入缓存的 token 数；provider 未提供时为空。 |
| `ChatChunk.content` / `reasoning_content` | 当前流分片中的可见文本与单独的推理文本。后者由部分 provider 支持，不等同于业务回答。 |
| `ChatChunk.tool_calls` / `finish_reason` / `index` / `usage` | 当前分片携带的工具调用增量、provider 结束原因、并行候选序号、token 统计。 |
| `ChatResponse.message` / `usage` / `finish_reason` / `reasoning_content` | 聚合后的 assistant 消息、整次请求 token 统计、provider 结束原因、可选推理文本。 |
| `LlmGenerationOutput.content` / `content_sha256` | driver 拼接后的主文本和该文本的 SHA-256；`usage`、`finish_reason` 对应整次流结果。 |

### 6.2 调用跟踪、上下文、checkpoint 和流事件

| 字段 | 含义 |
| --- | --- |
| `project_id` / `root_project_id` | 当前项目记录 ID；派生项目存在时，`root_project_id` 指原始根项目。 |
| `source_chapter_id` / `chapter_number` | 来源章节记录 ID；`chapter_number` 是项目内面向用户的章节序号，不是数据库 ID。 |
| `workflow_run_id` / `generation_run_id` | 工作流实例 ID；generation task 实例 ID。二者跟踪不同的运行边界。 |
| `LlmTraceContext.workflow` / `step_key` / `call_key` | 业务工作流名称 / 稳定步骤标识 / 可用于 checkpoint 对应单次 LLM 调用的键。 |
| `step_key` / `stage_key` | 工作流/生成任务中的稳定步骤名和阶段名。 |
| `call_key` / `call_order` | 一次 LLM 调用在 checkpoint 中的稳定键和执行次序。 |
| `input_fingerprint_sha256` | 规范化输入、prompt/context 的摘要，用于判断 checkpoint 是否仍对应当前输入。 |
| `context_plan_json` / `context_pack_refs_json` | 内部记录的上下文规划 / 本次选用上下文包的引用；是 trace 元数据，不进入 provider 请求体。 |
| `ChatRequest.trace_context` / `context_manifest_json` | 内部调用追踪上下文 / 候选上下文 manifest；都不会作为供应商请求字段发送。manifest 只有被规划器选中的包内容会渲染到模型消息。 |
| `source_snapshot` / `source_snapshot_sha256` | 本次任务依据的源内容版本快照；其摘要用于检查源内容是否变化。 |
| `record_id` / `expected_revision` | 要更新的业务记录 ID 和调用方期望的当前版本号，用于并发更新校验。 |
| `prompt_name` / `prompt_renderer` / `variables` | prompt 模板名、模板渲染器、模板变量键值表。prompt 名通常也决定输出 parser/schema。 |
| `temperature` / `cancel_token` | 本次模型采样配置；取消信号句柄。 |
| `restore_from_checkpoint` / `restore_required` | 是否尝试复用已完成 checkpoint；恢复是强制要求还是允许重新生成。 |
| `call_order` / `sequence` / `index` | 业务调用次序、传输事件序号、provider 候选/流片段序号；不能互换使用。 |
| `stream_id` / `offset` / `delta` / `chunk_index` | 前端文本通道名、该通道中的当前位置、这次追加的文本、分片序号。 |
| `attempt` / `task_id` / `emitted_at_ms` | generation task 重试次数、任务 ID、事件产生时间戳。 |
| `LlmContextManifest.schema_version` | manifest 格式版本。 |
| `LlmContextManifest.packs[]` | 候选上下文包列表；`scope_kind` 表示作用域类型，`pack_kind` 表示包用途，`source_refs_json` 指源记录，`config_hash` 标识构建配置，`required` 标识不能舍弃的包，`stable_prefix` 区分稳定前缀与可变部分。是否发送及发送哪个版本由预算规划决定。 |
| `variants[]` / `compression_level` / `canonical_content_json` | 同一包可供选择的压缩版本及各版本的规范 JSON 内容；每包最多选一个版本，不会将所有版本一起发送。 |
| `dynamic_user_content` / `output_contract` | 每次调用变化的用户输入文本、提供给模型的输出约束文本。 |
| `LlmGenerationRequest.request` / `stream_id` / `cancellation` | 实际模型请求、输出事件归属的文本流名、该请求的取消令牌。 |
| `checkpoint {writer,session,metadata}` / `usage_base` | 是否保存流进度及其 writer/session/调用元数据；生成前已有的累计 usage 基数。 |
| `ChatJsonStreamInput.llm` / `request` / `prompt_name` | 流 provider、发送的请求、用于错误上下文和阶段识别的 prompt 名。 |
| `truncated_message` / `cancelled_message` | 输出被 token 上限截断时、调用被取消时返回给业务层的错误说明。 |
| `structured_progress` / `on_delta` | 可选字段级 JSON 进度定义；可选的原始文本分片回调。 |
| `VideoPromptRenderInput.step` / `progress` / `chunks` | 当前视频提示步骤（位置、总数、提示文字、进度 span）；进度事件回调；模型文本分片回调。 |
| `VideoPromptRenderInput.service` | 编排服务依赖句柄，用来读取配置、运行模型调用和写 checkpoint；不是发给模型的字段。 |
| `VideoPromptRenderInput.llm_call_trace` | 当前 workflow run、call key、输入摘要的追踪信息，附加到内部 LLM trace。 |
| `CapabilityTaskEvent.task_id` / `event_kind` / `sequence` / `payload` | sidecar 任务 ID、事件类型、该任务内序号、事件数据。`progress` 表示心跳/进度，`chunk` 表示模型流分片，`completed`/`failed` 表示终态。 |
| `GenerationTaskEvent.schema_version` / `task_id` / `attempt` / `sequence` / `emitted_at_ms` / `event` | 前端 generation 事件合同版本、任务 ID、重试次数、事件序号、发出时间和具体事件内容。 |
| `UsageEvent.usage` / `cumulative` | 本次模型调用的 token 使用量；任务从启动到当前累计的 token 使用量。 |

### 6.3 Sidecar 封套字段

| 字段 | 含义 |
| --- | --- |
| `ServiceRequest.request_id` | 一次 sidecar 请求的关联 ID。 |
| `trace_context.trace_id` / `parent_span_id` | 全链路 trace ID、上游 span ID。 |
| `schema_version.protocol` / `database` | 服务协议版本、可选数据库 schema 版本。 |
| `user_id` / `project_id` / `root_project_id` | 发起用户、当前项目、根项目的可选 scope。 |
| `runtime_config_snapshot` | 本次运行所用配置快照；不是模型 prompt 字段。 |
| `secret_policy.scope` | 凭据转发范围：无 secret、仅本机 sidecar、或可信远端。 |
| `allow_system_secret` / `allow_user_secret` / `allow_upload_token` | 是否允许分别转发系统密钥、用户密钥、上传令牌。 |
| `capability_version` | 目标 sidecar capability 的合同版本。 |
| `payload` | capability 专属请求体；LLM 请求时为 `LlmGatewayPayload`。 |
| `LlmGatewayPayload.provider` / `request` | provider 配置和标准化聊天请求。 |
| `LlmGatewayProviderConfig.provider` / `base_url` / `model` | provider 类型、服务地址、模型名。 |
| `api_key` / `timeout_secs` / `extra_body_json` | 可空 API key、请求超时秒数、需要额外合并到 provider 请求体的 JSON。 |
| `ServiceResponse.status` / `trace_id` / `usage` / `artifact_refs` / `payload` / `error` | 服务成功/失败状态、关联 trace、服务级 usage、产生的 artifact 引用、成功结果、错误信息。 |
| `ServiceError.code` / `message` / `retryable` | 错误类别、可读错误文本、是否可重试。 |

### 6.4 生成、蓝图、工作流和大纲字段

#### 6.4.1 Generation driver 与生成结果

| 字段 | 含义 |
| --- | --- |
| `LlmGenerationOutput.content` | driver 收到并拼接后的模型主文本；是否是正文、JSON 或提示词由对应调用点决定。 |
| `content_sha256` | `content` 的摘要，用于校验、追踪或 checkpoint 内容一致性。 |
| `ParsedBlueprintOptimization.raw` | 模型返回的完整优化 JSON。 |
| `style` / `full_synopsis` / `world_setting` / `characters` / `relationships` | 蓝图中可被此次优化替换的文风、完整梗概、世界观、人物表、关系表；缺失字段表示此次未提供该项修改。 |
| `thought` / `public_update` | Agent 决策的内部分析文本 / 可展示给用户的进度说明。 |
| `action` / `arguments` | Agent 下一步控制动作 / 该动作的参数对象；`think` 继续推理，`run_tool` 调用工具，`final` 结束并返回结果。 |
| `response` / `decision` / `restored` | 包装后的模型响应 / 校验后的决策 / 是否从 checkpoint 恢复。 |
| `StartVisualPlanPromptGenerationInput.meta` | generation task 的并发、进度、取消等启动元数据。 |
| `source_chapter_id` / `record_id` / `expected_revision` | 视觉规划所依据的章节 ID、目标视觉计划记录 ID、写入时要求匹配的记录版本。 |
| `segment_id` / `user_description` | 目标镜头片段 ID / 用户为该片段补充的自然语言要求。 |
| `VisualPlanPromptCandidate.plan_revision` / `plan_sha256` | 候选所属视觉计划版本和计划内容摘要。 |
| `VisualPlanPromptCandidate.prompt` / `user_description` / `model` | 生成后待确认的绘图提示词 / 用户原始要求 / 使用的模型名。 |
| `InspirationResponse.ai_message` / `next_question` / `progress_summary` | 面向用户的回答 / 下一轮需要用户补充的问题 / 当前已收集信息的摘要。 |
| `ui_control.type` / `options` / `placeholder` | 前端应显示的交互控件类型 / 可选项列表 / 文本输入提示语。 |
| `InspirationOption.id` / `label` / `description` / `key_elements` | 选项稳定 ID、显示标题、选项说明、该选项代表的 2–3 个关键要素。 |
| `conversation_state` / `is_complete` | 可跨轮保留的问答状态 JSON / 是否已达到完成条件；完成态还需提供 `chapter_count` 和 `genre`。 |

#### 6.4.2 WorkflowLlmOnce 与 AdaptationPlan

| 字段 | 含义 |
| --- | --- |
| `WorkflowLlmOnceInput.workflow` / `stage_key` / `step_key` | workflow 类型、阶段身份、阶段内稳定步骤键。 |
| `call_order` / `prompt_name` / `user_prompt` | workflow 中的顺序号、system prompt 模板名、该次调用的 user prompt 内容。 |
| `project_id` / `root_project_id` / `workflow_run_id` | 当前项目、派生项目的根项目、工作流运行实例。 |
| `AdaptationPlanV1.schema_version` / `target` | 改编计划合同版本 / 目标媒介类型。 |
| `intention` | 这份改编计划要实现的总体戏剧目的。 |
| `decisions[]` | 对原作保留、删除、合并或调整的决策。 |
| `AdaptationDecision.description` / `reason` / `source_span_ids` | 具体改编决定 / 作出决定的原因 / 支撑决定的原文片段 ID。 |
| `units[]` | 将来源内容拆分后的改编单元，供后续分镜、剧本或页面流程使用。 |
| `AdaptationUnit.id` / `origin` / `title` | 单元稳定 ID / 单元来自原作或新增内容的来源类别 / 单元标题。 |
| `location` / `participants` | 发生地点 / 参与人物。 |
| `dramatic_question` / `objective` / `obstacle` | 单元的核心悬念 / 人物要完成的目标 / 阻碍目标的因素。 |
| `entry_state` / `exit_state` | 单元开始时和结束时人物、局势或关系的状态。 |
| `beats[]` | 单元内推动戏剧变化的最小事件/动作序列。 |
| `AdaptationBeat.id` / `kind` / `is_climax` | 节拍 ID / 节拍类型 / 是否是高潮节拍。 |
| `importance` / `action` / `reaction` | 节拍的重要程度 / 发生的动作 / 动作引发的反应。 |
| `dialogue_intent` | 对白要传达的意图，而非对白逐字稿。 |
| `AdaptationOrigin` / `AdaptationImportance` | 内容来源类别 / 该节拍在整体改编中的重要级别；具体枚举由领域合同限定。 |

#### 6.4.3 Galgame 输出

| 字段 | 含义 |
| --- | --- |
| `GalgameBranchDesign.choices` | 可供玩家选择的剧情分支列表。 |
| `after_beat_ref` | 分支插入位置所对应的来源章节、改编单元和节拍。 |
| `question` / `options` | 向玩家提出的问题 / 该问题的候选选项。 |
| `branch_id` / `option_text` / `branch_title` | 分支稳定 ID / 玩家看到的选项文字 / 选中后分支章节标题。 |
| `branch_objective` / `dramatic_action` / `reaction` | 分支剧情目标 / 该分支要发生的关键行动 / 行动后的戏剧反应。 |
| `player_knowledge` / `returns_to` | 进入该分支后玩家获得的信息 / 分支结束后回到的来源改编单元。 |
| `scene_lines` / `branch_scene_lines` | 主线路径的场景台词 / 选择分支里的台词。 |
| `scene_index` / `branch_id` | 对应场景的序号 / 对应的分支 ID。 |
| `lines[]` / `index` / `line_type` | 场景中的台词行列表 / 行序号 / 对白、旁白或其他行类型。 |
| `origin` / `beat_ref` | 台词内容来自原作还是新增 / 台词所改编的来源节拍。 |
| `speaker_name` / `speaker_profile_id` / `text` | 显示说话人名称 / 关联的角色档案 ID / 台词文本。 |
| `presentation.visible_cast` / `enter` / `exit` | 当前画面可见角色 / 本行登场的角色 ID / 本行退场的角色 ID。 |
| `focus_character_profile_id` / `hold` / `player_knowledge` | 镜头焦点角色 / 是否保持当前演出画面 / 本行后玩家应掌握的信息。 |
| `backgrounds[]` / `background_note` / `scene_profile_id` | 场景背景建议 / 背景画面描述 / 对应场景档案 ID。 |
| `event_cgs[]` / `line_index` / `cg_note` / `importance` | 事件 CG 规划 / 触发 CG 的台词行 / CG 内容说明 / CG 优先级。 |

#### 6.4.4 Import Reverse、蓝图与大纲

| 字段 | 含义 |
| --- | --- |
| `ReverseEngineerInput.target` / `content_sample` / `auto_context` / `context_limit` | 要反推的对象类型 / 手工提供的文本样本 / 是否从项目自动抽取上下文 / 自动上下文的 token/内容规模上限。 |
| `ReverseEngineeringCandidate.state` / `warnings` / `result` / `applied` | 候选当前状态 / 处理提示或告警 / 模型生成的候选数据 / 已应用到项目后的结果（尚未应用时为空）。 |
| blueprint `title` / `genre` / `style` / `tone` / `target_audience` | 蓝图标题 / 题材 / 文风 / 情绪基调 / 目标读者。 |
| `one_sentence_summary` / `full_synopsis` | 一句话故事概述 / 带起承转合的完整梗概。 |
| `total_chapters` / `needs_part_outlines` | 计划总章数 / 是否需要先生成分部大纲。 |
| `world_setting` | 世界观结构，包含规则、科技/魔法、地点、阵营、历史、社会文化、冲突和谜团。其具体子字段见 2.8。 |
| `character_type` / `gender` / `age` | 人物叙事类型 / 性别描述 / 年龄描述。 |
| `appearance` / `personality` / `background` | 外貌 / 性格 / 背景经历。 |
| `motivation` / `arc_summary` / `relationship_to_protagonist` | 行动动机 / 人物弧线概述 / 与主角的关系。 |
| `arc_stages` | 人物弧线阶段，如起点、创伤、欲望、需求、秘密、转折和终点。 |
| `relationship_type` / `character_a` / `character_b` / `description` | 关系类别 / 关系两端人物 / 关系描述。 |
| `strength` / `evolution` | 关系强度 / 关系在故事开始、冲突、转折和结尾的发展。 |
| `outline.chapters[]` / `chapter_number` | 章节概要列表 / 章节序号；在普通章节大纲流程中由系统按目标范围补入。 |
| `title` / `summary` / `beats` / `key_scenes` | 章节标题 / 章节摘要 / 推进剧情的节拍 / 关键场景。 |
| `pacing_score` / `foreshadowing_setup` / `foreshadowing_payoff` | 节奏评分 / 本章埋下的伏笔 / 本章回收的伏笔。 |
| `characters_involved` / `word_count_target` | 本章涉及人物 / 目标字数。 |
| `part_number` / `start_chapter` / `end_chapter` | 分部序号 / 覆盖的第一章 / 覆盖的最后一章。 |
| `theme` / `key_events` / `character_focus` | 分部主题 / 本部分关键事件 / 按角色组织的成长或剧情重点。 |
| `total_chapters` / `chapters_per_part` / `requested_part_count` | 全书目标章数 / 每部分章数 / 用户指定的分部数量。 |

#### 6.4.5 视觉规划与立绘提示词

| 字段 | 含义 |
| --- | --- |
| `meta` | generation task 的事件、取消、进度等启动上下文。 |
| `segment_id` / `user_description` | 目标镜头片段 / 用户对该片段的补充描述。 |
| `record_id` / `expected_revision` / `plan_revision` / `plan_sha256` | 视觉计划记录 ID、写入前期望版本、候选使用的计划版本、计划内容摘要。 |
| `VisualPlanPromptCandidate.prompt` / `model` | 待确认并用于图片生成的 prompt / 生成该 prompt 的模型名。 |
| `PortraitPromptSpec.profile_name` / `portrait_type` | 人物名称 / 立绘类别（例如正面、背面、侧面或单张立绘，具体由枚举限定）。 |
| `appearance` / `gender` / `age` / `personality` / `background` | 人物外貌、性别、年龄、性格和背景资料。 |
| `profile_context` / `project_context` | 人物档案的附加上下文 / 项目级设定上下文。 |
| `art_style` / `expression` / `existing_portrait_count` | 绘图风格要求 / 表情要求 / 已有立绘数量，用于控制变化或续生成。 |
| `normalize_portrait_prompt_response` 的返回字符串 | 可直接交给图像生成模型的 prompt 正文；该调用不要求模型返回 JSON 字段。 |

### 6.5 角色、场景与物体字段

#### 角色冲突检查

| 字段 | 含义 |
| --- | --- |
| `project_id` / `name` | 要检查的项目 / 角色名称。 |
| `start_chapter` / `end_chapter` | 可选的检查范围起点 / 终点章节。 |
| `chapter_number` / `field` | 冲突发生的章节 / 发生差异的角色资料字段。 |
| `expected` / `actual` | 从既有上下文得出的预期值 / 当前章节事实中观察到的值。 |
| `expected_source_id` / `actual_source_id` | 两个值分别引用的来源记录或证据 ID。 |
| `severity` | 冲突严重度：`minor` 轻微、`moderate` 中等、`major` 重大。 |

#### 角色档案同步

| 字段 | 含义 |
| --- | --- |
| `profile_id` / `profile_name` | 角色档案数据库 ID / 用于 prompt 展示的角色名。 |
| `start_chapter` / `target_chapter` | 本次扫描范围起点 / 要更新到的章节。 |
| `selected_chapter_numbers` / `chapter_data` | 实际选入同步的章节序号 / 这些章节的正文与分析上下文。 |
| `baseline_state` / `previous_steps` | 同步开始前的角色状态快照 / 前序 LLM 阶段已生成的数据。 |
| `events[].chapter_number` / `event_type` / `summary` | 事件发生章节 / 事件分类 / 事件简要说明。 |
| `related_characters[]` / `evidence_anchor_id` | 事件涉及人物 / 支持判断的正文锚点 ID。 |
| `attribute_changes[].assertion` | 变化属于正文观察 (`observed`) 还是推断 (`inferred`)；推断只允许用于 implicit 类属性。 |
| `category` / `key` / `operation` | 属性类别 (`explicit/implicit/social`) / 属性名 / 本阶段只允许新增、修改（`add/modify`）；删除由 `deletion_candidates` 单独提出。 |
| `new_value` / `change_description` / `event_cause` | 变更后的属性值 / 变更摘要 / 触发该变化的事件。 |
| `chapter_states[].state_summary` | 角色在该章结束时的整体状态。 |
| `current_location` / `current_goal` / `emotional_state` | 章节结束时的位置、当前目标和情绪状态。 |
| `behaviors[].behavior_type` / `behavior_description` | 行为分类 / 本章观察到的具体行为。 |
| `original_text_anchor_id` / `behavior_tags[]` | 行为依据的正文锚点 / 行为标签。 |
| `impact_analysis` / `related_characters[]` | 行为对人物弧线或关系的影响 / 相关人物。 |
| `deletion_candidates[].category` / `key` / `reason` | 建议删除的属性类别、属性键和删除理由。 |
| `old_value` / `evidence` | 由系统依据 baseline 和 source span 派生，不由模型输出。 |

#### 隐性属性复审与分类

| 字段 | 含义 |
| --- | --- |
| `attribute_key` / `current_value` | 被评估的隐性属性键 / 当前保存的属性值。 |
| `conform_count` / `non_conform_count` / `conform_rate` | 支持该属性的行为数 / 不支持该属性的行为数 / 支持比例。 |
| `threshold_reached` | 支持比例是否达到触发复审的阈值。 |
| `evidence[]` | 可供复审模型判断的既有行为证据；每项含行为 ID、章节、类型、描述、原文、标签和分类。 |
| `decision` / `reasoning` | 处理动作 (`keep/modify/delete`) / 作出该动作的说明。 |
| `new_value` | 修改后的属性值；modify 必须提供，keep/delete 必须为 null。 |
| `supporting_behavior_ids[]` | 支持复审结论的行为 ID；只能引用输入 `evidence[]`，系统据此生成 `evidence_summary`。 |
| `classifications` | 以当前隐性属性名为 key 的对象；value 表示本次行为符合 (`conform`) 或不符合 (`non-conform`) 该属性。 |
| `suggested_new_attributes[] {key,value}` | 当前属性集中没有、但本次行为提示的新属性建议；证据由系统回填。 |

#### 角色发现、场景档案和物体发现

| 字段 | 含义 |
| --- | --- |
| `DiscoverCharacterProfilesInput.project_id` / `chapter_number?` | 要扫描的项目 / 可选单章筛选。 |
| `characters[].name` / `aliases[]` | 标准角色名 / 正文中出现的别名。 |
| `importance` / `role` | 故事重要度 (`primary/secondary/minor`) / 角色叙事职责。 |
| `explicit_attributes` / `implicit_attributes` / `social_attributes` | 正文直述属性 / 由行为推断的属性 / 人际与社会关系属性。 |
| `evidence_anchor_id` | 支持该人物识别或属性判断的正文锚点。 |
| `created[]` / `existing[]` | 新建的角色档案 / 匹配到已有档案或未创建的候选。 |
| `base_attributes` / `current_state` / `character_arc` | 稳定人物画像 / 当前状态 / 跨章节变化轨迹。 |
| `voice_fingerprint` / `appearance` / `background` / `personality` | 语言习惯摘要 / 外观 / 经历 / 性格。 |
| `attribute_counts {explicit,implicit,social}` | 新建档案中三类属性的数量。 |
| `status` / `chapters_analyzed` / `chapter_numbers` / `discovered_count` | 任务状态 / 实际分析章数 / 分析的章节序号 / 新发现角色数。 |
| `scene_ref` / `scene_profile_id` | 本次输出内稳定的场景引用键 / 可复用的既有场景档案 ID。 |
| `scene_type` / `importance` / `parent_scene_id` | 场景类别 / 剧情重要度 / 层级结构中的父场景 ID。 |
| `base_description` / `visual_description` / `spatial_layout` / `atmosphere` | 场景总体描述 / 可见外观 / 空间布置 / 氛围感受。 |
| `entry_points[]` / `rules[]` / `evidence_anchor_ids[]` | 场景出入口 / 场景内稳定规则 / 支持场景档案的正文锚点。 |
| `objects[].object_type` / `importance` | 物体类别 / 故事重要度。 |
| `visual_description` / `material_and_state` | 外观描述 / 材质及当前状态。 |
| `story_usage` / `visual_usage` / `generation_usage` | 剧情功能 / 画面用途 / 后续图像生成用途。 |
| `location_context` / `first_seen_chapter` / `last_seen_chapter` | 物体的位置关系 / 首次出现章节 / 最近出现章节。 |
| `image_prompt_seed` / `evidence` | 系统拼出的图像提示词种子 / 系统从锚点还原的原文证据，不由模型填写。 |

### 6.6 Manga pipeline 字段

| 合同/字段 | 含义 |
| --- | --- |
| `prompt_name` / `project_id` | 本阶段 prompt 模板键 / 所属项目 ID。 |
| `workflow_run_id` / `call_key` / `call_order` | 整条 Manga pipeline 的 run ID / 阶段 checkpoint 键 / 执行顺序。 |
| `restore_from_checkpoint` | 是否允许复用该阶段之前保存的模型响应。 |
| `variables` / `user_content` / `temperature` | prompt 模板变量 / 实际来源上下文 manifest / 本次模型采样温度。 |
| `stream_id` / `stream_reporter` | 前端显示的流通道名 / 将模型 delta 推送到生成任务的回调。 |
| `MangaPreprocessingV3.schema_version` | 漫画预处理合同版本，当前为 `manga_preprocessing_v3`。 |
| `source_events.events[]` | 原文中的剧情、人物状态或空间变化事件。 |
| `event_id` / `type` / `description` / `participants[]` / `source_span_ids[]` | 事件唯一 ID / 类型 / 事实描述 / 参与者 / 原文证据。 |
| `characters` | 按规范角色名索引的外貌、性格、身份及来源资料。 |
| `speech.dialogues[]` / `speech.narrations[]` | 被选中的对白和旁白；分别通过 `source_event_id` 关联原文事件。 |
| `visual_anchors.scenes[]` | 场景地点、时间、氛围、天气、光线和空间描述；通过 `source_event_ids[]` 关联事件。 |
| `items.items[]` | 关键物品及视觉特征；`first_appearance_source_event_id` 指向首次出现事件。 |
| `AdaptationPlanV1.page_count` | 漫画改编计划确定的章节页数；分页规划必须严格输出相同数量的页面。Galgame 计划不包含该字段。 |
| `MangaChapterComicSynthesisV1.chapter_summary` / `mood_progression[]` | 章节漫画化重点 / 情绪递进；不决定页数。 |
| `MangaPageAdaptationV1.visual_intent` | 本页希望读者看到或感受到的核心画面意图。 |
| `continuity_requirements[]` | 本页必须遵守的视觉或叙事连续性约束。 |
| `retained_details[]` / `omitted_details[]` | 从原作保留到本页的细节 / 为适配漫画而省略的细节。 |
| `beat_refs[]` | 本页覆盖的改编节拍引用；每个引用由 `{unit_id,beat_id}` 标识。 |
| `MangaPageV4.page_number` / `beat_refs[]` | 页码 / 本页表达的节拍集合。 |
| `role` / `pacing` / `importance` / `reading_flow` | 本页在情节中的功能 / 阅读节奏 / 页面重要度 / 视线阅读路径。 |
| `page_goal` / `visual_thesis` | 本页要完成的叙事目标 / 统摄全页构图的视觉主张。 |
| `entry_state` / `exit_state` | 页面开始和结束时故事状态，用于页间连续性。 |
| `dominant_panel_number` / `panels[]` | 本页主导画格编号 / 本页画格列表。 |
| `MangaPanelV2.panel_number` / `beat_refs[]` | 画格序号 / 画格呈现的节拍。 |
| `function` / `size` / `shot_scale` / `camera_angle` | 画格叙事功能 / 版面尺寸 / 摄影景别 / 摄影机观察角度。 |
| `participants[]` / `composition` | 画格内人物 / 画面布局描述。 |
| `observable_action` / `reaction` | 画格中可见动作 / 动作引发的反应。 |
| `spatial_relationship` / `continuity_from_previous` | 人物或物体之间的空间关系 / 与上一画格的连续关系。 |
| `text_layers[]` | 叠加在画格上的对白、旁白、思绪或拟声文字。 |
| `MangaTextLayerV2.layer_index` / `kind` | 同一画格中文字层次序 / 文字类型 (`dialogue/thought/narration/sfx`)。 |
| `speaker` / `text` / `placement` / `avoid_covering[]` | 说话人 / 显示文字 / 文字气泡位置 / 不能被气泡遮挡的画面元素。 |
| `MangaPageSlotsV1.schema_version` / `pages[]` | 页面槽位合同版本 / 规划出的页面列表。 |
| `MangaPageSlotV1.page_number` / `content` | 页面序号 / 为该页分配的剧情内容。 |
| `MangaPageContentV1.summary` / `source_span_ids[]` / `beat_refs[]` | 页面内容摘要 / 来源正文片段 / 覆盖节拍。 |
| `action` / `reaction` / `dialogue_intent` | 页面核心动作 / 动作反应 / 对白要传达的意图。 |
| `MangaPageDrawingPromptV2.schema_version` / `page_number` / `image_prompt` | 绘图 prompt 合同版本 / 目标页码 / 交给图像模型的整页画面描述。 |
| `MangaPageReferenceDiscoveryV1.references[]` | 系统从候选图片中选出的单页参考图。 |
| `asset` / `target_kind` / `target_name` | 图片素材标识 / 参考对象类型 / 参考对象名称。 |
| `description` / `usage` | 素材外观描述 / 本页沿用该素材的具体方式。 |
| `reference_images[].source` | `system` 表示系统发现；`user` 表示用户补充。两组描述都会进入绘图提示词。 |
| `source_span_ids[]` | 支撑事件、角色、场景、物品或页面内容的正文片段 ID。 |

### 6.7 拍摄剧本字段

| 字段 | 含义 |
| --- | --- |
| `scene_index` / `sequence_index` / `narrative_unit_index` | 最终场景序号 / 剧本排列序号 / 对应的叙事单元序号。 |
| `scene_identity` / `visual_narrative_unit_id` | 稳定场景身份键 / 对应导演视觉计划单元的 ID。 |
| `timeline_position` / `intercut_group_id` / `editing_mode` | 故事时间位置 / 交叉剪辑分组键 / 连贯、交叉剪辑、蒙太奇等剪辑方式。 |
| `transition_contracts[]` | 本场景与后续叙事单元之间必须保留的剪辑/声音/动作约束。 |
| `to_narrative_unit_index` / `transition_type` | 转场目标单元序号 / 转场形式。 |
| `preserved_action` / `preserved_sound` / `preserved_knowledge` | 转场中需要延续的动作、声音和观众已知信息。 |
| `required_cut_point` | 必须执行剪辑的内容或时间节点。 |
| `title` / `slugline` | 场景标题 / screenplay 场景行（内外景、地点、时段等）。 |
| `location` / `time_of_day` | 场景地点 / 故事发生的时间段。 |
| `participants[]` / `focal_character` | 出场人物集合 / 本场主要观察对象。 |
| `source_span_ids[]` / `user_addition_ids[]` | 对应正文片段 ID / 用户在导演编排中新增的描述 ID。 |
| `dramatic_question` / `scene_objective` / `scene_obstacle` | 本场悬念 / 人物目标 / 阻碍目标的因素。 |
| `relationship_before` / `relationship_after` | 场景前后关键人物关系状态。 |
| `audience_knowledge_before[]` / `audience_knowledge_after[]` / `withheld_information[]` | 观众进入场景时已知、离开时新增、以及刻意暂不揭示的信息。 |
| `viewpoint.mode` / `anchor_character` / `information_strategy` | 叙事视点类型 / 视点锚定人物 / 管理观众可获取信息的方式。 |
| `spatial_continuity.continuity_intent` / `primary_axis` / `screen_direction` / `reorientation_strategy` | 空间连续性目标 / 主轴线 / 屏幕方向 / 有意改变方向时的交代方式。 |
| `sound_strategy` | 本场声音设计意图。 |
| `entry_state` / `exit_state` | 场景开始和结束时的故事状态。 |
| `shots[]` | 本场镜头设计列表。 |
| `shot_index` / `shot_id` / `visual_shot_id` | 镜头顺序号 / 拍摄脚本镜头 ID / 对应视觉计划镜头 ID。 |
| `planned_start_ms` / `planned_end_ms` | 镜头在源时间线中的计划起止毫秒位置。 |
| `visual_constraints` / `coverage` | 从视觉计划继承的硬约束 / 镜头覆盖目标。 |
| `narrative_purpose` / `subject` | 镜头在叙事中的用途 / 镜头主要拍摄对象。 |
| `shot_size` / `camera_angle` / `camera_height` / `lens_intent` | 景别 / 机位角度 / 机位高度 / 镜头焦段或透视意图。 |
| `camera_movement` / `composition` / `spatial_anchors` | 运镜方式 / 画面构图 / 维持空间识别的关键位置或物件。 |
| `blocking` / `performance_beats` | 人物在空间内的调度 / 演员动作和反应节拍。 |
| `dialogue_cues` / `sound` | 镜头中的对白提示 / 音效、环境声或音乐设计。 |
| `camera_start_state` / `camera_end_state` | 镜头开始和结束时相机位置、朝向或运动状态。 |
| `start_state` / `end_state` | 镜头主体动作开始和结束时的可见状态。 |
| `transition_in` / `transition_out` / `continuity_requirements[]` | 进入/离开本镜头的剪辑连接 / 镜头必须满足的连续性条件。 |
| `ScreenplayUnitAdaptationV1.schema_version` / `narrative_unit_index` | 表演改编合同版本 / 改编对应的叙事单元。 |
| `performance_facts[].fact_index` / `shot_id` | 表演事实序号 / 必须承载该事实的目标镜头。 |
| `actor` / `objective` / `obstacle` / `visible_action` | 表演者 / 行动目标 / 行动阻力 / 镜头可见的动作。 |
| `reaction` / `dialogue_intent` / `entry_state` / `exit_state` | 动作反应 / 对白意图 / 表演开始和结束状态。 |
| `required` | 该表演事实是否必须在镜头中覆盖。 |

### 6.8 章节分析字段

章节分析四阶段的字段合同见 2.9。来源标记必须引用本次章节正文上下文提供的标记，不能自行编造数据库 ID。

| 字段 | 含义 |
| --- | --- |
| `characters[]` / `locations[]` / `items[]` | 本章识别到的人物、地点和物品实体；每项的 `name` 是实体名称，`source_span_ids[]` 指向正文中的依据片段。 |
| `event_candidates[].event_ref` / `type` | 事件候选的阶段内引用键 / 事件类别；类别由 validator 限定为 battle、revelation、relationship、discovery、decision、death、arrival 或 departure。 |
| `description` / `importance` / `involved_characters[]` | 事件内容摘要 / 相对叙事重要度（high、medium、low）/ 事件涉及的人物名称。 |
| `event_candidates[].source_span_ids[]` | 支持该事件候选的正文片段标记。 |
| `foreshadowing_candidates[].foreshadowing_ref` / `description` | 伏笔候选的阶段内引用键 / 对伏笔内容的描述。 |
| `category` / `priority` / `related_entities[]` | 伏笔类别（人物秘密、情节反转、物品谜团或世界规则）/ 重要级别（high、medium、low）/ 相关人物、地点或物品。 |
| `original_text_span_id` | 伏笔最初出现的正文片段标记；系统用它还原证据文本。 |
| `world_facts[].field` / `subject` / `statement` | 世界事实所属的蓝图字段类别 / 事实描述的对象 / 关于该对象的陈述。 |
| `assertion` / `world_facts[].source_span_ids[]` | 陈述的认识状态（直接观察、推断、转述、假设或回忆）/ 支撑该陈述的正文片段。 |
| `metadata_additions.tags[]` / `tone` / `timeline_marker` | 建议加入章节元数据的标签 / 章节基调 / 章节在故事时间线上的标记。 |
| `summaries.compressed` / `one_line` / `keywords[]` | 用于上下文压缩的章节摘要 / 一句话摘要 / 检索和概览用关键词。 |
| `character_states` 对象的 key | 角色名称；其值记录该角色在本章结束时的状态。 |
| `location` / `status` / `changes[]` / `emotional_state` | 角色所在位置 / 当前处境或状态 / 本章发生的变化说明列表 / 本章结束时的情绪。 |
| `foreshadowing.planted[]` | 本章新埋下或明确呈现的伏笔；`id?` 可关联已有伏笔，`description` 描述内容，`original_text_span_id` 指证据位置，其余类别、优先级和实体引用沿用事实候选语义。 |
| `foreshadowing.resolved[]` / `progressed[]` | 本章已揭示/回收的伏笔 / 本章发生状态变化或推进的既有伏笔。 |
| `id` / `resolution` / `source_span_ids[]` | 要更新的既有伏笔 ID / 本章揭示或推进了什么 / 对应正文证据标记；ID 必须来自输入的待处理伏笔。 `resolved` 的最终状态由系统设为 `resolved`。 |
| `progressed[].status` | 本次更新后的伏笔状态：`pending`（待揭示）、`partially_revealed`（部分揭示）、`resolved`（已回收）或 `abandoned`（已弃置）。 |
| `foreshadowing.tensions` | prompt 示例中用于表达尚未解决的张力；当前 validator 没有固定其元素字段，因此不能视作稳定 schema。 |
| `key_events[]` | 本章最终确认并用于章节时间线/事件材料的关键事件；其 `type`、`description`、`importance`、`involved_characters[]`、`source_span_ids[]` 与事实候选同义。 |
| `ChapterAnalysisStageSpec.stage` / `step_key` / `step_order` | 当前分析阶段名 / 稳定工作流步骤键 / 四阶段中的执行顺序；属于调度元数据，不进入模型结果。 |
| `context` / `restored_progress` / `fields` / `progress_span` | 阶段说明文本 / 从 checkpoint 恢复时采用的进度点 / 用于结构化进度显示的字段清单 / 阶段映射到总进度的区间；均为内部执行元数据。 |

### 6.9 视频提示词字段

模型响应的外层结构和提示词字段见 2.10。帧/镜头编号、时长、画幅、引用素材 ID 等有些由已有片段或默认值补齐；只有 parser 明确要求的字段才属于模型 JSON 合同。

| 字段 | 含义 |
| --- | --- |
| `frames[]` / `shots[]` / `segments[]` | 分别表示帧提示词、视频镜头提示词、视频生成片段规划列表。 |
| `frame_number` / `shot_number` | 已规划帧或片段的序号；由调用器按既有计划和响应条目顺序推导，不要求模型回传。 |
| `prompt_kind` / `prompt_output_language` | 视频提示词优化所针对的提示词类别 / 系统要求并校验的输出语言。 |
| `title` / `scene_summary` | 帧、镜头或片段的标题 / 概括其画面和叙事内容的摘要。 |
| `frame_prompt` / `visual_prompt` / `chat_prompt` | 静帧画面描述 / 视频画面主体与视觉风格描述 / 面向 chat-video provider 的完整提示词。 |
| `motion_prompt` / `negative_prompt` | 对主体动作、镜头或时间变化的描述 / 需要避免生成的内容或缺陷。 |
| `blocking_state` / `lighting_state` | 帧中的人物位置、朝向和动作状态 / 帧中的光源、方向或光照状态；二者是任意 JSON 子树，内层结构没有统一合同。 |
| `storyboard_panels[]` | 分解关键时刻的有序故事板格列表。 |
| `panel_index` / `moment` / `visual_goal` / `composition` / `continuity_note` | 故事板格序号 / 该格表现的瞬间 / 该格的视觉目标 / 画面布局 / 与前后格保持的连续性说明。 |
| `dialogue_cues` | 镜头相关的对白提示；当前为任意 JSON 值，具体子字段随调用 prompt 而变。 |
| `source_shot_id` / `covered_beat_ids[]` | 片段来源镜头的稳定 ID / 该片段覆盖的叙事节拍 ID。 |
| `blocking_summary` / `spatial_anchors[]` | 片段中人物行动和位置调度的摘要 / 用于维持空间可读性的锚点及其使用方式。 |
| `emotion_action_goals[]` | 角色层面的情绪与可见动作目标；每项的 `character`、`emotion`、`visible_action` 分别是角色、情绪和画面应呈现的动作。 |
| `video_intent.hook` / `audience_effect` / `rhythm` / `visual_priority` | 开场吸引点 / 目标观众感受 / 节奏意图 / 画面信息的优先级。 |
| `emotional_arc.start` / `turn` / `end` / `function` | 情绪起点 / 变化转折 / 结束状态 / 该情绪变化对片段叙事的作用。 |
| `performance_plan.movement_path` / `body_performance` / `gaze_and_hands` | 人物移动路线 / 身体动作表演 / 视线和手部动作设计。 |
| `pause_or_reaction` / `camera_relationship` | 停顿或反应安排 / 表演动作与摄影机位置、运动之间的关系。 |
| `time_blocks[]` | 按片段时间分段的表演与摄影安排；`start_sec`、`end_sec` 为区间边界，`camera` 和 `subject_action` 描述该区间的镜头与主体动作，`continuity_target` 是该段需保持的连续性目标。 |
| `action_script[]` | 角色动作清单；`character`、`body`、`movement`、`hands`、`gaze`、`timing` 分别描述角色、身体、移动、手部、视线和动作时机。 |
| `material_coverage.required_entities[]` / `missing_or_weak_references[]` / `risk_note` | 必须出现在片段中的实体 / 尚未覆盖或引用较弱的实体 / 规划风险说明。 |
| `duration_secs` / `aspect_ratio` / `reference_image_ids[]` / `segment_numbers[]` | 视频时长 / 画幅比例 / 输入或关联的参考图 ID / 关联片段序号；它们在部分流程中由系统而非模型补入。 |

### 6.10 Agent 与 Runtime 字段

主回合 #27 由 Agent 自己组装历史投影，消息依次为：角色/能力域/协议/资源 scope/执行模式及 Skill 元数据目录的 system、可选旧摘要 system、控制面完成规则 system、可选委派验收条件 system、配对后的保留历史、当前 phase/goal/parent run 与未完成 delegation/execution 状态的最后一条 user。历史通常取旧摘要覆盖边界之后的消息，同时保留本 turn 的 user；tool 消息由保存的 observation 或执行回执投影，不保证是完整原始工具结果。旧摘要明确标为参考数据；Skill 正文、业务资料和记忆通过工具按需读取，不是全部预先放入 system。[提示词与消息](../crates/agent/a4n-agent-runtime/src/engine/prompt.rs#L1)

| Agent 协议 | 请求/响应合同 |
| --- | --- |
| `native_tool_calling` | 顶层 `tools` 放当前工具 JSON Schema，`response_format=None`；模型通过 assistant `tool_calls` 发起工具调用，工具结果按调用 ID 回填；最终回答为文本。 |
| `json_compat` | 顶层 `tools=None,tool_choice=None`，`response_format.type=json_object`；工具 schema 写在 system。模型根对象只允许 `{action,arguments}`：`think` 的 arguments 为 `{thought,public_update}`，`run_tool` 为 `{tool,arguments}`，`final` 为 `{reply}`。[协议实现](../crates/agent/a4n-agent-runtime-contract/src/protocol.rs#L134) |

这里的 `json_compat` 与主表 #5 优化 Agent 的 `{thought,public_update?,action,arguments}` 是两套合同，不能互换。主回合温度为 `0.2`，先配对工具调用/结果并按模型窗口拟合；`context_manifest_json=None`。

压缩 #28 使用独立 system 规则和一条 user JSON：`{previous_summary,current_request,messages,covered_through_sequence}`。`current_request[]` 是本 turn 的 user 消息 `{message_id,content}`；`messages[]` 是旧覆盖边界之后、预算内可完整装入的历史批次 `{message_id,message:ChatMessage,execution_id}`，最新整批保留在主回合。历史不能完整装入投影，或占用达到拟合后可用输入容量的 80% 时尝试压缩；摘要来源预算按模型总窗口的 65% 控制。不是把全部历史重新发送。[历史选择](../crates/agent/a4n-agent-runtime/src/engine/history.rs#L117)

压缩请求温度 `0.1`，输出 token 上限先取模型配置与 4096 的较小值，再按窗口拟合结果调整。返回 schema version 必须为 2，覆盖边界必须等于本批输入边界，禁止返回 tool calls；数组最多 64 项、每项最多 4000 字符。[摘要构造与校验](../crates/agent/a4n-agent-runtime/src/engine/context.rs#L28)

| 字段 | 含义 |
| --- | --- |
| `chat_stream.user_id` / `config_id` / `request` | 发起调用的用户 ID / 可选模型配置 ID / 实际发送的标准化 `ChatRequest`。 |
| `chat_stream.snapshot` | 本次运行可用的运行时配置快照；用于解析模型配置，不是对话消息，也不直接作为模型文本发送。 |
| `tools[].type` / `function.name` / `function.description` / `function.parameters` | 工具声明类型（function）/ 可调用工具名 / 给模型看的工具用途说明 / 参数 JSON Schema。参数 schema 由当前可用工具动态提供，没有一个适用于全部工具的固定字段集合。 |
| `AgentContextSummary.schema_version` | 摘要数据合同版本；当前模型压缩调用要求为 2。 |
| `current_goal` / `constraints[]` / `decisions[]` | 当前 Agent 任务目标 / 仍有效的硬约束 / 已作出的重要决策或已确认解释。 |
| `facts_resources[]` / `completed[]` / `pending[]` | 应保留的事实、资源标识/版本和工具结果 / 已完成事项 / 尚待完成事项。 |
| `tool_state[]` | 后续运行需要的工具执行状态或结果摘要。 |
| `covered_through_sequence` | 摘要覆盖到的最后一条持久化消息 ID。名称虽含 sequence，它不是 SSE 事件序号；返回值必须与本次读取历史的边界一致。 |
| `ReadImageInput.project_id` / `image_ids[]` / `chapter_id` | 图片所属项目 / 要交给视觉模型分析的图片记录 ID 列表 / 可选章节范围校验条件。 |
| `instruction` | 视觉检查目标；为空时 Runtime 使用内置默认检查说明。 |
| `vision_images[].media_type` / `data_base64` | 每张已读取图片的 MIME 类型 / 图片字节的 Base64 内容。 |
| `ReadImageResult.project_id` / `image_ids[]` / `analysis` | 本次分析的项目 / 实际分析图片 ID / 模型返回并经长度限制后的视觉评估文本。 |
| `CanvasTextAssetDraftInput.project_id` / `llm_config_id` | 文本素材所属项目 / 可选的模型配置选择。 |
| `instruction` / `source_asset_id` / `source_version` | 生成草稿的用户指令 / 可选参考素材 ID / 参考素材的指定版本；素材 ID 和版本必须同时提供或同时省略。 |
| `CanvasTextAssetDraftSummary.content` | 模型生成的可编辑正文；该调用只消费模型文本，不要求 JSON 输出。 |
| `CanvasTextGenerationLlmCall.project_id` / `workflow_run_id` / `config_id` / `request` | Canvas 所属项目 / 发起调用的 Canvas workflow run / 可选模型配置 / 实际聊天请求。返回完整 `ChatResponse` 给上层 Canvas 流程。 |

### 6.11 媒体生成与连通性探测字段

本节接口使用图像/视频生成合同，不计入 49 个聊天 LLM 入口；字段释义用于区分文本模型调用和媒体模型调用的数据边界。

| 字段 | 含义 |
| --- | --- |
| `ImageGenerationRequest.prompt` / `width` / `height` / `model` | 完整图像生成提示词 / 输出像素宽度 / 输出像素高度 / 请求使用的模型名。 |
| `reference_images[].label` / `data_uri` | 参考图的用途或稳定标签 / 格式为 `data:image/*;base64,...` 的图片数据 URI。 |
| `stream` / `modalities` / `n` / `size` / `messages[]` | 是否使用流式响应（此图像请求固定为 false）/ provider wire body 要求的输出模态 / 生成数量 / 宽高组合 / 承载提示词和参考素材的消息。 |
| `messages[].content[].type` / `text` / `image_url.url` | 多模态消息片段类型（`text` 或 `image_url`）/ 图片生成提示词文本片段 / 参考图 data URI。没有参考图时，`content` 是普通文本字符串。 |
| `ImageGenerationResult.bytes` / `format` / `model` | 解码后的图像原始字节 / 文件格式扩展名 / provider 实际使用的模型名。 |
| `choices[0].message.images` / `content` | 图像接口从 provider 首个候选消息中优先读取的图像列表 / 可回退提取的文本或图像数据。提取到的 URL/base64 被下载或解码为结果字节。 |
| `VideoGenerationRequest.prompt` / `negative_prompt` | 视频主体提示词 / 可选的负面约束。 |
| `width` / `height` / `duration_secs` / `fps` / `model` | 输出尺寸 / 视频时长 / 每秒帧数 / 请求模型名。 |
| `media.type` | 输入媒体模式：`none`、`frames` 或 `references`。 |
| `media.first` / `last` / `references` | 首帧图 / 可选尾帧图 / 辅助参考图；只用于 `frames` 模式。 |
| `media.images[]` / `videos[]` / `audio[]` | `references` 模式下的参考图片、视频和音频来源。图片源可为 URL 或 Host artifact 引用；视频和音频当前是字符串来源。 |
| `VideoMediaSource.source` / `url` / `artifact` | 图片来源类型；`url` 是 URL 来源，`artifact` 是 Host 管理的 artifact 引用。使用 artifact 时由 `ArtifactRef` 描述文件。 |
| `ArtifactRef.artifact_id` / `artifact_kind` / `ownership` | 媒体文件稳定 ID / 媒体类别 / 文件由 Host、Sidecar 还是远端拥有。 |
| `owner_user_id` / `project_id` / `content_type` | 可选所属用户 / 可选所属项目 / 媒体 MIME 类型。 |
| `sha256` / `byte_size` / `location` / `expires_at` | 内容摘要 / 文件字节数 / 本地路径或上传/下载 URL / 可选过期时间。 |
| `ArtifactLocation` | 文件的可访问位置，合同枚举为 `local_path`、`upload_url` 或 `download_url`；视频生成输入校验要求 Host 图片 artifact 使用项目目录内的 `local_path`。 |
| `reference_image_urls` / `image` / `last_frame` | provider wire body 中的参考图 URL 列表 / 首帧输入 / 尾帧输入；它们由统一 `media` 结构按 provider 适配。 |
| `choices[0].message.video` / `videos` / `content` | 视频接口从 provider 首个候选消息中解析的单个视频 / 视频列表 / 可能包含视频链接或数据的文本内容。 |
| `VideoGenerationResult.bytes` / `format` / `provider_job_id` / `metadata` | 解码后的视频字节 / 文件格式 / provider 异步任务 ID（同步或无任务 ID 时为空）/ provider 返回的附加 JSON 元数据；metadata 没有跨 provider 固定子 schema。 |
| `LlmProviderConfig.id` / `name` / `provider` / `base_url` / `model` | 配置记录 ID / 展示名称 / provider 类型 / API 服务地址 / 模型标识。 |
| `api_key` / `timeout_secs` / `max_concurrent` | 可空 provider 凭证 / 单次请求超时秒数 / 该配置允许的并发调用上限。凭证值不应写入此文档或日志。 |
| `max_context_tokens` / `max_output_tokens` | 模型上下文窗口上限 / 模型输出 token 上限。 |
| `extra_body_json` / `agent_protocol` | 需要合并到 provider 请求体的扩展 JSON（内部键由 provider 决定）/ Agent 使用的协议适配模式；可用协议值由 provider adapter 支持范围决定。 |
| `pricing.currency` | 下列费率使用的货币单位。 |
| `llm_input_per_million` / `llm_output_per_million` | 每百万输入 token / 输出 token 的费率。 |
| `llm_cache_read_per_million` / `llm_cache_write_per_million` | 每百万缓存读取 token / 缓存写入 token 的费率。 |
| `embedding_input_per_million` / `image_per_generation` / `video_per_second` | 每百万 embedding 输入 token / 每次图像生成 / 每秒视频生成的费率。费率字段为空表示未配置该项价格。 |
| `LlmProbeOutput.response_preview` | 连通性测试模型返回的文本预览。 |
| `usage` / `finish_reason` | 本次探测返回的 token 用量 / provider 结束原因；nullable 值可以是 null，但 sidecar 合同要求键存在。 |

### 6.12 原始响应与 checkpoint 结果字段

这些字段描述调用器如何保留模型结果，不能与模型 JSON 本身混为一谈。

| 字段 | 含义 |
| --- | --- |
| `response_content` / `response_text` | provider 返回并拼接的原始文本；它可能是 JSON，但此字段本身尚未表示 JSON 已解析或通过业务校验。 |
| `StructuredLlmStreamResult.response_content` | 结构化流 driver 收集的完整原始文本；`usage` 和 `finish_reason` 是同一次流的统计与结束原因。 |
| `RawCallResult.restored` | 结果来自已完成 checkpoint 时为 true；此时没有重新发起模型请求。 |
| `restored_parsed_output` | checkpoint 中已保存的解析结果；为空时调用器需从原始响应重新解析和校验。 |
| `response_only_checkpoint` | checkpoint 只保留原始响应文本，没有可直接复用的 parsed output。 |
| `upgraded_contract` | checkpoint 使用了旧版拍摄时长合同，重解析时需要执行到当前版本的字段升级和归一化。 |
| `MangaPromptGenerationTrace.request_json` / `messages_json` | 运行时预算/manifest 重写前保存的业务请求 trace / 当时的消息数组。`request_json` 含 manifest 摘要和 trace，未保存完整 manifest 原文；`messages_json` 尚不包含后来选入的上下文包，二者都不是最终供应商 HTTP body。[保存位置](../crates/host/a4n-orchestrator/src/manga/pipeline/prompt/llm/request.rs#L70) |
| `input_fingerprint_json` | 参与 checkpoint 匹配的 prompt 名、项目作用域和规范化请求内容；相同调用键只有输入指纹匹配才可恢复。 |
| `MangaPromptGenerationTrace.response_text` | 本次模型原始文本，随后交给严格 JSON parser 和该 prompt 的 validator。 |
| `RenderedVideoPromptTrace.request` / `response_text` | 视频业务构造并留存的 `ChatRequest` / 模型原始返回文本；请求仍会经过运行时上下文规划和供应商适配。 |

`context_aware_chat_request_trace_json()` 在普通请求序列化上加 `context_manifest_summary` 及 trace 的 `context_plan/context_pack_refs`，不等同于保留完整内部字段的 `snapshot_value()`。[trace 实现](../crates/llm/a4n-llm-trace/src/lib.rs#L56) 核对最终模型输入时，应结合运行时选包结果和供应商 `request_body_preview`；不能只凭上游 `messages_json` 判断上下文是否缺失。[运行时请求记录](../crates/host/a4n-runtime/src/dynamic_llm/provider/preparation.rs#L100)
