use crate::actions::json_contract::{
    optional_string, positive_i64_array, reject_unknown_fields, required_string, string_array,
};
use crate::actions::text_anchor_contract::anchor_quotes_with_chapter_numbers;
use crate::{
    actions::llm_request::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT,
        chat_request_with_workflow_run_id, context_aware_system_user_json_llm_request,
    },
    state::AppState,
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::models::asset::object_profile;
use a4n_core::models::chapter::chapter;
use a4n_core::models::project::text_anchor;
use a4n_engine::llm::provider::ChatRequest;
use a4n_engine::prompt::request::build_json_task_user_message;
use a4n_services::asset::{
    OBJECT_TYPES, ObjectProfileDraft, ObjectProfileUpsertResult, normalize_object_name,
    normalize_object_type_value,
};
use a4n_services::generation::{
    RecordCompletedGenerationRunInput, context_aware_chat_request_trace_json,
};
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use a4n_services::profile_text::normalize_importance_value;
use a4n_services::source::source_content_sha256;
use a4n_services::structured_llm_stream::{
    ChatJsonStreamInput, StructuredLlmProgress, chat_json_stream_with_progress,
};
use a4n_services::structured_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
use a4n_services::text_anchor::{
    EVIDENCE_ROLE_FIRST_SEEN, PROFILE_KIND_OBJECT, ProfileEvidenceInput, text_anchor_prompt_values,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use tokio_util::sync::CancellationToken;

const OBJECT_DISCOVERY_PROGRESS_TOTAL: u32 = 100;
const OBJECT_DISCOVERY_LLM_SPAN: ProgressSpan = ProgressSpan::new(10, 75);
const OBJECT_DISCOVERY_CALL_KEY: &str = "object_discovery.llm";
const OBJECT_DISCOVERY_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("name", "物体名称", 1),
    FieldMilestone::new("aliases", "别名", 1),
    FieldMilestone::new("object_type", "物体类型", 1),
    FieldMilestone::new("importance", "重要性", 1),
    FieldMilestone::new("visual_description", "外观描述", 1),
    FieldMilestone::new("material_and_state", "材质状态", 1),
    FieldMilestone::new("story_usage", "叙事用途", 1),
    FieldMilestone::new("visual_usage", "视觉用途", 1),
    FieldMilestone::new("generation_usage", "生成用途", 1),
    FieldMilestone::new("location_context", "位置关系", 1),
    FieldMilestone::new("evidence_anchor_ids", "证据锚点", 1),
];
pub(crate) type ObjectDiscoveryProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

pub(crate) struct ObjectDiscoveryProgress<'a> {
    reporter: Option<ObjectDiscoveryProgressReporter<'a>>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ObjectDiscoveryProgress<'a> {
    pub(crate) fn new(
        reporter: Option<ObjectDiscoveryProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    fn progress(&self, current: u32, total: u32, message: &str) {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
    }

    fn check_cancelled(&self, message: &str) -> Result<(), String> {
        if self.cancel_token.is_some_and(|token| token.is_cancelled()) {
            Err(message.to_string())
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ObjectDiscoveryPreviewStep {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) prompt_name: Option<String>,
    pub(crate) state: String,
    pub(crate) note: Option<String>,
    pub(crate) request: ChatRequest,
}

struct ObjectDiscoveryRequestContext {
    chapters: Vec<chapter::Model>,
    text_anchors: Vec<text_anchor::Model>,
    request: ChatRequest,
}

#[derive(Debug, Clone, Copy)]
enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

#[derive(Debug)]
struct ParsedObjectProfileDraft {
    draft: ObjectProfileDraft,
    evidence_anchor_ids: Vec<i64>,
}

pub(crate) async fn discover_object_profiles_for_chapters(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    progress: Option<&ObjectDiscoveryProgress<'_>>,
) -> Result<Value, String> {
    report_object_discovery_progress(
        progress,
        0,
        "正在读取可分析章节和已有物体档案",
        "物体档案发现已停止",
    )?;
    let context = build_object_discovery_request(
        state,
        project_id,
        chapter_number,
        ChapterAnchorMode::EnsureCurrent,
    )
    .await?;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: object_discovery_business_key(project_id, chapter_number),
            workflow_kind: "object_discovery".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some(
                chapter_number
                    .map(|value| format!("chapter:{value}"))
                    .unwrap_or_else(|| "all".to_string()),
            ),
            input_fingerprint_json: Some(object_discovery_workflow_fingerprint(
                project_id,
                chapter_number,
                &context.chapters,
                &context.text_anchors,
                &context.request,
            )),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(OBJECT_DISCOVERY_CALL_KEY.to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        report_object_discovery_progress(
            progress,
            OBJECT_DISCOVERY_LLM_SPAN.start(),
            "正在调用模型识别关键物体",
            "物体档案发现已停止",
        )?;
        let fingerprint_request = context.request.clone();
        let mut llm_request =
            chat_request_with_workflow_run_id(fingerprint_request.clone(), workflow_run_id);
        let analyzed_chapters = context
            .chapters
            .iter()
            .map(|chapter| chapter.chapter_number)
            .collect::<BTreeSet<_>>();
        let call_fingerprint_json =
            object_discovery_call_fingerprint_json(project_id, chapter_number, &llm_request)?;
        let call_fingerprint_sha256 =
            llm_call_checkpoint_fingerprint_sha256(&call_fingerprint_json)
                .map_err(|err| err.to_string())?;
        if let Some(trace_context) = llm_request.trace_context.as_mut() {
            trace_context.call_key = Some(OBJECT_DISCOVERY_CALL_KEY.to_string());
            trace_context.input_fingerprint_sha256 = Some(call_fingerprint_sha256);
        }
        let (parsed_drafts, response_content) = if let Some(checkpoint) =
            load_completed_object_discovery_call_checkpoint(
                state,
                workflow_run_id,
                &call_fingerprint_json,
            )
            .await?
        {
            report_object_discovery_progress(
                progress,
                80,
                "物体档案发现已从断点恢复，正在写入物体档案",
                "物体档案发现已停止",
            )?;
            let (parsed_drafts, parsed_output) = parse_and_validate_object_discovery_output(
                &checkpoint.response_content,
                &analyzed_chapters,
                &context.chapters,
                &context.text_anchors,
            )?;
            if parsed_output != checkpoint.parsed_output {
                return Err(format!(
                    "{} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致",
                    OBJECT_DISCOVERY_CALL_KEY
                ));
            }
            (parsed_drafts, checkpoint.response_content)
        } else {
            let response_content = chat_json_stream_with_progress(
                ChatJsonStreamInput {
                    llm: state.llm.as_ref(),
                    request: llm_request.clone(),
                    prompt_name: "object_discovery",
                    truncated_message: "物体档案发现输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“角色/档案发现”输出档位，或减少本次章节上下文后重试",
                    structured_progress: Some(StructuredLlmProgress {
                        spec: StructuredJsonProgressSpec {
                            root_array_key: "objects",
                            item_label: "物体",
                            expected_items: None,
                            milestones: OBJECT_DISCOVERY_FIELDS,
                        },
                        span: OBJECT_DISCOVERY_LLM_SPAN,
                    }),
                    cancel_token: progress.and_then(|progress| progress.cancel_token),
                    cancelled_message: "物体档案发现已停止",
                },
                |current, message| {
                    report_object_discovery_progress(
                        progress,
                        current,
                        message,
                        "物体档案发现已停止",
                    )
                },
            )
            .await?;

        report_object_discovery_progress(
            progress,
            80,
            "模型已返回，正在校验物体发现结果",
            "物体档案发现已停止",
        )?;
        let (parsed_drafts, parsed_output) = parse_and_validate_object_discovery_output(
            &response_content,
            &analyzed_chapters,
            &context.chapters,
            &context.text_anchors,
        )?;
        save_completed_object_discovery_call_checkpoint(
            state,
            workflow_run_id,
            call_fingerprint_json,
            &response_content,
            parsed_output,
        )
        .await?;
        (parsed_drafts, response_content)
    };
    let discovered_count = parsed_drafts.len();

    report_object_discovery_progress(progress, 90, "正在写入物体档案", "物体档案发现已停止")?;
    let request_json =
        serde_json::to_value(&llm_request).map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let messages_json = serde_json::to_value(&llm_request.messages)
        .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
    let result = state
        .object_profile
        .upsert_many_with_generation_run(
            project_id,
            parsed_drafts
                .iter()
                .map(|item| item.draft.clone())
                .collect(),
            RecordCompletedGenerationRunInput {
                project_id,
                workflow: "object_discovery".to_string(),
                step_key: "discover_object_profiles_for_chapters".to_string(),
                title: "物体档案发现".to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: None,
                prompt_template_names_json: Some(json!(["object_discovery"])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(messages_json),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "chapter_number": chapter_number,
                    "analyzed_chapters": analyzed_chapters,
                    "text_anchor_count": context.text_anchors.len(),
                })),
                source_refs_json: Some(json!({
                    "chapters": context.chapters.iter().map(|chapter| json!({
                        "id": chapter.id,
                        "chapter_number": chapter.chapter_number,
                        "title": chapter.title,
                    })).collect::<Vec<_>>(),
                })),
                media_refs_json: None,
                request_json: Some(request_json),
                response_summary_json: Some(json!({
                    "response_chars": response_content.chars().count(),
                })),
                targets: Vec::new(),
            },
        )
        .await
        .map_err(|err| err.to_string())?;
    bind_object_profile_evidence(state, project_id, &parsed_drafts, &result).await?;
    let value = object_discovery_result(state, context.chapters, discovered_count, result).await?;
    report_object_discovery_progress(
        progress,
        OBJECT_DISCOVERY_PROGRESS_TOTAL,
        "物体档案发现已完成",
        "物体档案发现已停止",
    )?;
    Ok(value)
    }
    .await;

    if let Err(status_err) =
        finalize_object_discovery_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "物体档案发现完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

fn object_discovery_business_key(project_id: i64, chapter_number: Option<i32>) -> String {
    match chapter_number {
        Some(chapter_number) => format!("object_discovery:{project_id}:chapter:{chapter_number}"),
        None => format!("object_discovery:{project_id}:all"),
    }
}

fn object_discovery_workflow_fingerprint(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
) -> Value {
    json!({
        "workflow": "object_discovery",
        "contract_version": 1,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "chapters": chapters.iter().map(|chapter| json!({
            "id": chapter.id,
            "chapter_number": chapter.chapter_number,
            "content_sha256": source_content_sha256(&chapter.content),
            "analysis_data": chapter.analysis_data,
        })).collect::<Vec<_>>(),
        "text_anchors": text_anchors.iter().map(|anchor| json!({
            "id": anchor.id,
            "source_chapter_id": anchor.source_chapter_id,
            "source_id": anchor.source_id,
            "quote_hash": anchor.quote_hash,
            "start_offset": anchor.start_offset,
            "end_offset": anchor.end_offset,
        })).collect::<Vec<_>>(),
        "system_prompt_sha256": request.messages.first()
            .map(|message| source_content_sha256(&message.content)),
        "temperature": request.temperature,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
    })
}

fn object_discovery_call_fingerprint_json(
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "object_discovery",
        "call_key": OBJECT_DISCOVERY_CALL_KEY,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "request": context_aware_chat_request_trace_json(request).map_err(|err| err.to_string())?,
    }))
}

struct ObjectDiscoveryCallCheckpoint {
    response_content: String,
    parsed_output: Value,
}

async fn load_completed_object_discovery_call_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    input_fingerprint_json: &Value,
) -> Result<Option<ObjectDiscoveryCallCheckpoint>, String> {
    let Some(checkpoint) = load_completed_llm_call_checkpoint(
        state.db.clone(),
        workflow_run_id,
        OBJECT_DISCOVERY_CALL_KEY,
        input_fingerprint_json,
    )
    .await
    .map_err(|err| err.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, OBJECT_DISCOVERY_CALL_KEY)
        .map_err(|err| err.to_string())?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            format!(
                "{}.response_content 必须是字符串",
                OBJECT_DISCOVERY_CALL_KEY
            )
        })?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| format!("{}.parsed_output 缺失", OBJECT_DISCOVERY_CALL_KEY))?;
    Ok(Some(ObjectDiscoveryCallCheckpoint {
        response_content,
        parsed_output,
    }))
}

async fn save_completed_object_discovery_call_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    input_fingerprint_json: Value,
    response_content: &str,
    parsed_output: Value,
) -> Result<(), String> {
    save_completed_llm_call_checkpoint(
        state.db.clone(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: OBJECT_DISCOVERY_CALL_KEY.to_string(),
            call_order: 1,
            input_fingerprint_json,
            checkpoint_json: json!({
                "output": parsed_output.clone(),
                "parsed_output": parsed_output,
                "response_content": response_content,
                "usage": null,
                "finish_reason": null,
            }),
            artifacts_json: None,
        },
    )
    .await
    .map_err(|err| err.to_string())
}

async fn finalize_object_discovery_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<Value, String>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    state
        .workflow_run
        .mark_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub(crate) async fn object_discovery_preview_steps(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    initial_state: &str,
    initial_note: Option<String>,
) -> Result<Vec<ObjectDiscoveryPreviewStep>, String> {
    let context = build_object_discovery_request(
        state,
        project_id,
        chapter_number,
        ChapterAnchorMode::RequireExistingCurrent,
    )
    .await?;
    let note = match initial_note {
        Some(note) => format!("{note}；分析章节数：{}", context.chapters.len()),
        None => format!("分析章节数：{}", context.chapters.len()),
    };
    Ok(vec![ObjectDiscoveryPreviewStep {
        key: "object_discovery".to_string(),
        title: "物体档案发现 LLM 请求".to_string(),
        prompt_name: Some("object_discovery".to_string()),
        state: initial_state.to_string(),
        note: Some(note),
        request: context.request,
    }])
}

async fn build_object_discovery_request(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    anchor_mode: ChapterAnchorMode,
) -> Result<ObjectDiscoveryRequestContext, String> {
    let chapters = select_object_discovery_chapters(state, project_id, chapter_number).await?;
    let existing_profiles = state
        .object_profile
        .list_by_project(project_id)
        .await
        .map_err(|err| err.to_string())?;
    let existing_prompt_profiles = existing_profiles
        .iter()
        .map(existing_object_profile_for_prompt)
        .collect::<Vec<_>>();
    let chapter_data = chapters
        .iter()
        .map(object_discovery_chapter_context)
        .collect::<Vec<_>>();
    let mut text_anchors = Vec::new();
    for chapter in &chapters {
        let anchors = match anchor_mode {
            ChapterAnchorMode::EnsureCurrent => {
                state
                    .text_anchor
                    .ensure_chapter_content_anchors(project_id, chapter.id)
                    .await
            }
            ChapterAnchorMode::RequireExistingCurrent => {
                state
                    .text_anchor
                    .read_current_chapter_content_anchors(project_id, chapter.id)
                    .await
            }
        }
        .map_err(|err| err.to_string())?;
        text_anchors.extend(anchors);
    }
    let system_prompt = state
        .engine
        .render("object_discovery", &HashMap::new(), Some(project_id))
        .map_err(|err| err.to_string())?;
    let stable_context = object_discovery_stable_context(
        &existing_prompt_profiles,
        &chapter_data,
        &chapters,
        &text_anchors,
    );
    let user_prompt = build_object_discovery_manifest_user_prompt()?;
    let chapter_ids = chapters
        .iter()
        .map(|chapter| chapter.id)
        .collect::<Vec<_>>();
    let text_anchor_ids = text_anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<Vec<_>>();
    Ok(ObjectDiscoveryRequestContext {
        chapters,
        text_anchors,
        request: context_aware_system_user_json_llm_request(
            system_prompt,
            ContextAwareJsonLlmRequestInput {
                project_id,
                workflow_run_id: None,
                workflow: "object_discovery".to_string(),
                step_key: "object_discovery.parsed".to_string(),
                scope_kind: "profile_sync".to_string(),
                pack_kind: PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT.to_string(),
                source_refs_json: Some(json!({
                    "chapter_number": chapter_number,
                    "chapter_ids": chapter_ids,
                    "text_anchor_ids": text_anchor_ids,
                })),
                config_json: json!({
                    "ledger_version": "object_discovery_context_v1",
                    "prompt_name": "object_discovery",
                }),
                stable_context_json: stable_context,
                dynamic_user_prompt: user_prompt,
                temperature: Some(0.2),
                max_tokens_profile: llm_token_profiles::CHARACTER_DISCOVERY.to_string(),
                call_key: Some(object_discovery_business_key(project_id, chapter_number)),
            },
        )?,
    })
}

async fn select_object_discovery_chapters(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, String> {
    if project_id <= 0 {
        return Err("物体档案发现 project_id 必须大于 0".to_string());
    }
    if chapter_number.is_some_and(|value| value <= 0) {
        return Err("物体档案发现章节号必须大于 0".to_string());
    }
    let mut chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .filter(chapter::Model::has_available_content)
        .collect::<Vec<_>>();
    if let Some(chapter_number) = chapter_number {
        chapters.retain(|chapter| chapter.chapter_number == chapter_number);
        if chapters.is_empty() {
            return Err(format!(
                "第 {chapter_number} 章没有可用于物体档案发现的有效正文"
            ));
        }
        return Ok(chapters);
    }
    if chapters.is_empty() {
        return Err(format!(
            "项目还没有可用于物体档案发现的章节: project_id={project_id}"
        ));
    }
    Ok(chapters)
}

fn build_object_discovery_manifest_user_prompt() -> Result<String, String> {
    let payload = object_discovery_payload(json!({
        "context_refs": {
            "source": format!("稳定上下文包 {PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT}")
        }
    }));
    build_object_discovery_prompt_from_payload(payload)
}

fn object_discovery_stable_context(
    existing_profiles: &[Value],
    chapter_data: &[Value],
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Value {
    json!({
        "existing_object_profiles": existing_profiles,
        "chapters": chapter_data,
        "text_anchors": text_anchor_prompt_values(chapters, text_anchors)
    })
}

fn object_discovery_payload(input: Value) -> Value {
    json!({
        "task": {
            "name": "object_discovery",
            "instruction": "从本次输入章节中识别可复用的关键物体档案；已有物体档案只用于同一物体判定"
        },
        "input": input,
        "output_contract": {
            "format": "json_object",
            "required_root_field": "objects",
            "object_scope": "返回本次输入章节中明确出现且可作为视觉素材复用的物体。已有物体也可以返回，后端会更新而非重复创建。",
            "object_type_values": [
                "prop",
                "tool",
                "weapon",
                "token",
                "vehicle",
                "clothing",
                "location_object",
                "environment_object",
                "other"
            ],
            "importance_values": ["primary", "secondary", "minor"],
            "object_fields": [
                "name",
                "aliases",
                "object_type",
                "importance",
                "visual_description",
                "material_and_state",
                "story_usage",
                "visual_usage",
                "generation_usage",
                "location_context",
                "evidence_anchor_ids"
            ],
            "object_type_rules": [
                "普通道具、线索物以外的一般物品用 prop，不要输出 item/object/thing",
                "设备、仪器、工具用 tool",
                "刀、枪、剑、斧等武器用 weapon",
                "信物、遗物、关键道具、象征物用 token，不要输出 key_item/relic/artifact",
                "车、船、火车等载具用 vehicle",
                "衣物、制服、配饰用 clothing",
                "门、窗、桌椅、床、货架等固定场景物件用 location_object",
                "碎石、水洼、烟雾、雾气、灯光等环境物件用 environment_object",
                "只有确实无法归类才用 other"
            ],
            "chapter_scope": "后端会根据 evidence_anchor_ids 所在章节推导 first_seen_chapter 和 last_seen_chapter；不要输出这两个字段",
            "strict_rules": [
                "只返回可解析的完整 JSON 对象，不要 Markdown 或解释文字",
                "不要把纯地点、组织、能力、事件或人物当作物体",
                "每个物体对象只允许输出 output_contract 列出的字段，不要添加额外字段",
                "evidence_anchor_ids 中的每个 ID 必须来自 input.text_anchors[].id",
                "evidence_anchor_ids 是核验用选择题，优先选择物体名称、外观或状态短语对应的 text_anchor；不要自己抄写原文，不要输出省略号证据",
                "location_context 只写简短位置语境文本；无法判断时输出 null，不要输出额外锚点 ID",
                "章节范围和图片提示词种子由系统推导，模型不要输出"
            ]
        }
    })
}

fn build_object_discovery_prompt_from_payload(payload: Value) -> Result<String, String> {
    build_json_task_user_message(
        "执行物体档案发现",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|err| format!("物体发现上下文序列化失败: {err}"))
}

fn object_discovery_chapter_context(chapter: &chapter::Model) -> Value {
    json!({
        "chapter_number": chapter.chapter_number,
        "chapter_title": chapter.title,
        "content": chapter.content,
        "analysis_data": chapter.analysis_data,
    })
}

fn existing_object_profile_for_prompt(profile: &object_profile::Model) -> Value {
    json!({
        "name": profile.name,
        "aliases": profile.aliases_json,
        "object_type": profile.object_type,
        "importance": profile.importance,
        "visual_description": profile.visual_description,
        "material_and_state": profile.material_and_state,
        "story_usage": profile.story_usage,
        "visual_usage": profile.visual_usage,
        "generation_usage": profile.generation_usage,
        "location_context": profile.location_context,
        "first_seen_chapter": profile.first_seen_chapter,
        "last_seen_chapter": profile.last_seen_chapter,
    })
}

#[cfg(test)]
fn parse_and_validate_object_discovery_content(
    content: &str,
    analyzed_chapters: &BTreeSet<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<ParsedObjectProfileDraft>, String> {
    let (drafts, _) = parse_and_validate_object_discovery_output(
        content,
        analyzed_chapters,
        chapters,
        text_anchors,
    )?;
    Ok(drafts)
}

fn parse_and_validate_object_discovery_output(
    content: &str,
    analyzed_chapters: &BTreeSet<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<(Vec<ParsedObjectProfileDraft>, Value), String> {
    let value = object_discovery_response_json(content)?;
    let drafts = parse_object_discovery_output_value(&value, chapters, text_anchors)?;
    validate_object_chapter_scope(&drafts, analyzed_chapters)?;
    Ok((drafts, value))
}

fn object_discovery_response_json(content: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        let mut message = format!("物体档案发现结果必须是 JSON 对象: {err}");
        if err.is_eof() {
            message.push_str("；响应在 JSON 闭合前结束，通常是模型输出被截断或 max_tokens 不足");
        }
        message
    })?;
    Ok(value)
}

fn parse_object_discovery_output_value(
    value: &Value,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<ParsedObjectProfileDraft>, String> {
    if !value.is_object() {
        return Err("物体档案发现结果必须是 JSON 对象".to_string());
    }
    reject_unknown_fields(value, "root", &["objects"])?;
    let objects = value
        .get("objects")
        .and_then(Value::as_array)
        .ok_or_else(|| "物体档案发现字段 objects 必须是数组".to_string())?;
    let mut seen = BTreeSet::new();
    let mut drafts = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        let draft = normalize_object(index, object, chapters, text_anchors)?;
        let key = draft.draft.name.trim().to_lowercase();
        if !seen.insert(key) {
            return Err(format!("objects[{index}] 与前序物体重复"));
        }
        drafts.push(draft);
    }
    Ok(drafts)
}

fn normalize_object(
    index: usize,
    object: &Value,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<ParsedObjectProfileDraft, String> {
    let path = format!("objects[{index}]");
    if !object.is_object() {
        return Err(format!("{path} 必须是 JSON 对象"));
    }
    let name = required_string(object, "name", &path)?;
    reject_unknown_fields(
        object,
        &path,
        &[
            "name",
            "aliases",
            "object_type",
            "importance",
            "visual_description",
            "material_and_state",
            "story_usage",
            "visual_usage",
            "generation_usage",
            "location_context",
            "evidence_anchor_ids",
        ],
    )?;
    let evidence_anchor_ids = positive_i64_array(object, "evidence_anchor_ids", &path, true)?;
    let evidence_with_chapters = anchor_quotes_with_chapter_numbers(
        &format!("{path}.evidence_anchor_ids"),
        &evidence_anchor_ids,
        chapters,
        text_anchors,
    )?;
    let first_seen_chapter = evidence_with_chapters
        .iter()
        .map(|anchor| anchor.chapter_number)
        .min()
        .ok_or_else(|| format!("{path}.evidence_anchor_ids 不能为空"))?;
    let last_seen_chapter = evidence_with_chapters
        .iter()
        .map(|anchor| anchor.chapter_number)
        .max()
        .ok_or_else(|| format!("{path}.evidence_anchor_ids 不能为空"))?;
    let evidence = evidence_with_chapters
        .iter()
        .map(|anchor| anchor.quote.clone())
        .collect::<Vec<_>>();
    let location_context = optional_string(object, "location_context", &path)?;
    let aliases = string_array(object, "aliases", &path, false)?;
    let object_type = object_type_string(object, "object_type", &path)?;
    let importance = importance_string(object, "importance", &path)?;
    let visual_description = required_string(object, "visual_description", &path)?;
    let material_and_state = required_string(object, "material_and_state", &path)?;
    let story_usage = required_string(object, "story_usage", &path)?;
    let visual_usage = required_string(object, "visual_usage", &path)?;
    let generation_usage = required_string(object, "generation_usage", &path)?;
    let image_prompt_seed = object_image_prompt_seed(
        &name,
        &visual_description,
        &material_and_state,
        &generation_usage,
    );
    Ok(ParsedObjectProfileDraft {
        evidence_anchor_ids,
        draft: ObjectProfileDraft {
            aliases,
            object_type,
            importance,
            visual_description,
            material_and_state,
            story_usage,
            visual_usage,
            generation_usage,
            location_context,
            first_seen_chapter,
            last_seen_chapter,
            evidence,
            image_prompt_seed,
            name,
        },
    })
}

fn object_image_prompt_seed(
    name: &str,
    visual_description: &str,
    material_and_state: &str,
    generation_usage: &str,
) -> String {
    format!(
        "{name}，{visual_description}，{material_and_state}，{generation_usage}，可复用物体素材设定图"
    )
}

fn object_type_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_object_type_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            OBJECT_TYPES.join("/")
        )
    })
}

fn importance_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_importance_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：primary/secondary/minor",
            text
        )
    })
}

fn validate_object_chapter_scope(
    drafts: &[ParsedObjectProfileDraft],
    analyzed_chapters: &BTreeSet<i32>,
) -> Result<(), String> {
    if analyzed_chapters.is_empty() {
        return Err("物体档案发现缺少已分析章节".to_string());
    }
    for (index, draft) in drafts.iter().enumerate() {
        if draft.draft.first_seen_chapter <= 0 || draft.draft.last_seen_chapter <= 0 {
            return Err(format!(
                "objects[{index}] first_seen_chapter/last_seen_chapter 必须大于 0"
            ));
        }
        if draft.draft.last_seen_chapter < draft.draft.first_seen_chapter {
            return Err(format!(
                "objects[{index}] last_seen_chapter 不能早于 first_seen_chapter"
            ));
        }
        if !analyzed_chapters.contains(&draft.draft.first_seen_chapter) {
            return Err(format!(
                "objects[{index}].first_seen_chapter 第 {} 章不在本次物体发现输入章节中",
                draft.draft.first_seen_chapter
            ));
        }
        if !analyzed_chapters.contains(&draft.draft.last_seen_chapter) {
            return Err(format!(
                "objects[{index}].last_seen_chapter 第 {} 章不在本次物体发现输入章节中",
                draft.draft.last_seen_chapter
            ));
        }
    }
    Ok(())
}

async fn bind_object_profile_evidence(
    state: &AppState,
    project_id: i64,
    drafts: &[ParsedObjectProfileDraft],
    result: &ObjectProfileUpsertResult,
) -> Result<(), String> {
    let profiles = result
        .created
        .iter()
        .chain(result.updated.iter())
        .map(|profile| (profile.normalized_name.clone(), profile.id))
        .collect::<HashMap<_, _>>();
    for draft in drafts {
        let normalized = normalize_object_name(&draft.draft.name).map_err(|err| err.to_string())?;
        let profile_id = profiles
            .get(&normalized)
            .copied()
            .ok_or_else(|| format!("物体档案写入结果缺少对象: {}", draft.draft.name))?;
        let mut refs = Vec::new();
        let mut seen = BTreeSet::new();
        for anchor_id in &draft.evidence_anchor_ids {
            if seen.insert((*anchor_id, EVIDENCE_ROLE_FIRST_SEEN)) {
                refs.push(ProfileEvidenceInput {
                    text_anchor_id: *anchor_id,
                    evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
                });
            }
        }
        state
            .text_anchor
            .merge_profile_evidence(project_id, None, PROFILE_KIND_OBJECT, profile_id, refs)
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn object_discovery_result(
    _state: &AppState,
    chapters: Vec<chapter::Model>,
    discovered_count: usize,
    result: ObjectProfileUpsertResult,
) -> Result<Value, String> {
    let mut created = Vec::with_capacity(result.created.len());
    for profile in &result.created {
        created.push(object_profile_result(profile));
    }
    let mut updated = Vec::with_capacity(result.updated.len());
    for profile in &result.updated {
        updated.push(object_profile_result(profile));
    }
    Ok(json!({
        "status": "discovered",
        "chapters_analyzed": chapters.len(),
        "chapter_numbers": chapters.iter().map(|chapter| chapter.chapter_number).collect::<Vec<_>>(),
        "discovered_count": discovered_count,
        "created": created,
        "updated": updated,
    }))
}

fn object_profile_result(profile: &object_profile::Model) -> Value {
    json!({
        "id": profile.id,
        "project_id": profile.project_id,
        "name": profile.name,
        "normalized_name": profile.normalized_name,
        "aliases_json": profile.aliases_json,
        "object_type": profile.object_type,
        "importance": profile.importance,
        "visual_description": profile.visual_description,
        "material_and_state": profile.material_and_state,
        "story_usage": profile.story_usage,
        "visual_usage": profile.visual_usage,
        "generation_usage": profile.generation_usage,
        "location_context": profile.location_context,
        "first_seen_chapter": profile.first_seen_chapter,
        "last_seen_chapter": profile.last_seen_chapter,
        "evidence_json": profile.evidence_json,
        "image_prompt_seed": profile.image_prompt_seed,
        "created_at": profile.created_at.to_string(),
        "updated_at": profile.updated_at.to_string(),
    })
}

fn report_object_discovery_progress(
    progress: Option<&ObjectDiscoveryProgress<'_>>,
    current: u32,
    message: &str,
    _cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.progress(current, OBJECT_DISCOVERY_PROGRESS_TOTAL, message);
        progress.check_cancelled(_cancelled_message)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_services::chapter::source_spans::source_spans_for_chapters;

    fn chapter_for_test() -> chapter::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter::Model {
            id: 1,
            project_id: 1,
            outline_id: None,
            chapter_number: 3,
            title: Some("旧书房".to_string()),
            content: "他在旧书房抽屉里摸到一枚青铜钥匙，钥匙柄部刻着残缺的星纹。".to_string(),
            word_count: 32,
            status: chapter::STATUS_DRAFT.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn anchors_for_test(chapters: &[chapter::Model]) -> Vec<text_anchor::Model> {
        let now = chrono::Utc::now().naive_utc();
        source_spans_for_chapters(chapters)
            .into_iter()
            .enumerate()
            .map(|(index, span)| text_anchor::Model {
                id: i64::try_from(index + 1).unwrap(),
                project_id: 1,
                source_chapter_id: Some(span.chapter_id),
                source_snapshot_id: None,
                source_snapshot_chapter_id: None,
                source_kind: "chapter_content".to_string(),
                source_id: span.chapter_id,
                paragraph_index: i32::try_from(span.sentence_index).unwrap(),
                start_offset: i32::try_from(span.char_start).unwrap(),
                end_offset: i32::try_from(span.char_end).unwrap(),
                quote: span.text,
                quote_hash: format!("hash-{}", index + 1),
                source_version_hash: "source-version".to_string(),
                created_at: now,
            })
            .collect()
    }

    fn anchor_id_containing(anchors: &[text_anchor::Model], text: &str) -> i64 {
        anchors
            .iter()
            .find(|anchor| anchor.quote.contains(text))
            .map(|anchor| anchor.id)
            .unwrap_or_else(|| panic!("missing text anchor containing {text}"))
    }

    #[test]
    fn parses_valid_object_discovery_output() {
        let chapters = vec![chapter_for_test()];
        let analyzed = BTreeSet::from([3]);
        let anchors = anchors_for_test(&chapters);
        let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");

        let content = serde_json::json!({
            "objects": [
                {
                    "name": "青铜钥匙",
                    "aliases": ["旧钥匙"],
                    "object_type": "token",
                    "importance": "primary",
                    "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                    "material_and_state": "青铜材质，边缘磨损",
                    "story_usage": "开启后续密室的关键线索",
                    "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                    "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                    "location_context": "旧书房抽屉里",
                    "evidence_anchor_ids": [key_anchor_id]
                }
            ]
        })
        .to_string();

        let drafts =
            parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
                .expect("valid object discovery output should parse");

        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].draft.name, "青铜钥匙");
        assert_eq!(drafts[0].draft.object_type, "token");
        assert_eq!(
            drafts[0].draft.location_context.as_deref(),
            Some("旧书房抽屉里")
        );
        assert_eq!(drafts[0].draft.first_seen_chapter, 3);
        assert_eq!(drafts[0].draft.last_seen_chapter, 3);
        assert_eq!(
            drafts[0].draft.evidence[0],
            "他在旧书房抽屉里摸到一枚青铜钥匙"
        );
        assert!(drafts[0].draft.image_prompt_seed.contains("青铜钥匙"));
        assert!(
            drafts[0]
                .draft
                .image_prompt_seed
                .contains("柄部刻着残缺星纹")
        );
        assert!(
            drafts[0]
                .draft
                .image_prompt_seed
                .contains("约束青铜钥匙星纹、磨损边缘和青铜材质")
        );
    }

    #[test]
    fn rejects_object_type_and_importance_aliases() {
        let chapters = vec![chapter_for_test()];
        let analyzed = BTreeSet::from([3]);
        let anchors = anchors_for_test(&chapters);
        let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");

        let content = serde_json::json!({
            "objects": [
                {
                    "name": "青铜钥匙",
                    "aliases": ["旧钥匙"],
                    "object_type": "key_item",
                    "importance": "high",
                    "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                    "material_and_state": "青铜材质，边缘磨损",
                    "story_usage": "开启后续密室的关键线索",
                    "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                    "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                    "location_context": "旧书房抽屉里",
                    "evidence_anchor_ids": [key_anchor_id]
                }
            ]
        })
        .to_string();

        let err =
            parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
                .expect_err("object enum aliases should fail visibly");

        assert!(err.contains("objects[0].object_type 无效"));

        let content = serde_json::json!({
            "objects": [
                {
                    "name": "青铜钥匙",
                    "aliases": ["旧钥匙"],
                    "object_type": "token",
                    "importance": "high",
                    "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                    "material_and_state": "青铜材质，边缘磨损",
                    "story_usage": "开启后续密室的关键线索",
                    "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                    "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                    "location_context": "旧书房抽屉里",
                    "evidence_anchor_ids": [key_anchor_id]
                }
            ]
        })
        .to_string();

        let err =
            parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
                .expect_err("importance aliases should fail visibly");

        assert!(err.contains("objects[0].importance 无效"));
    }

    #[test]
    fn rejects_unknown_object_type() {
        let chapters = vec![chapter_for_test()];
        let analyzed = BTreeSet::from([3]);
        let anchors = anchors_for_test(&chapters);
        let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");

        let content = serde_json::json!({
            "objects": [
                {
                    "name": "青铜钥匙",
                    "aliases": [],
                    "object_type": "memory",
                    "importance": "primary",
                    "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                    "material_and_state": "青铜材质",
                    "story_usage": "开启后续密室的关键线索",
                    "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                    "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                    "location_context": null,
                    "evidence_anchor_ids": [key_anchor_id]
                }
            ]
        })
        .to_string();

        let err =
            parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
                .expect_err("unknown object_type should fail");

        assert!(err.contains("objects[0].object_type 无效"));
        assert!(err.contains("token"));
    }

    #[test]
    fn rejects_unknown_object_importance() {
        let chapters = vec![chapter_for_test()];
        let analyzed = BTreeSet::from([3]);
        let anchors = anchors_for_test(&chapters);
        let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");

        let content = serde_json::json!({
            "objects": [
                {
                    "name": "青铜钥匙",
                    "aliases": [],
                    "object_type": "token",
                    "importance": "legendary",
                    "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                    "material_and_state": "青铜材质",
                    "story_usage": "开启后续密室的关键线索",
                    "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                    "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                    "location_context": null,
                    "evidence_anchor_ids": [key_anchor_id]
                }
            ]
        })
        .to_string();

        let err =
            parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
                .expect_err("unknown importance should fail");

        assert!(err.contains("objects[0].importance 无效"));
        assert!(err.contains("primary"));
    }

    #[test]
    fn rejects_unknown_object_evidence_anchor_id() {
        let content = serde_json::json!({
            "objects": [
                {
                    "name": "不存在的戒指",
                    "aliases": [],
                    "object_type": "token",
                    "importance": "secondary",
                    "visual_description": "银色戒指",
                    "material_and_state": "银质",
                    "story_usage": "线索",
                    "visual_usage": "保持银色戒指作为线索物的画面连续性",
                    "generation_usage": "约束银色戒指外观和银质材质",
                    "location_context": null,
                    "evidence_anchor_ids": [999]
                }
            ]
        })
        .to_string();
        let chapters = vec![chapter_for_test()];
        let anchors = anchors_for_test(&chapters);
        let analyzed = BTreeSet::from([3]);

        let err =
            parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
                .expect_err("unknown anchor should fail");

        assert!(err.contains("evidence_anchor_ids 无效"));
        assert!(err.contains("text_anchor_id 不在本次输入候选中"));
    }

    #[test]
    fn object_evidence_anchor_ids_avoid_rewritten_source_text() {
        let mut chapter = chapter_for_test();
        chapter.content = "他用冷水冲洗左眼眶和左手残肢，疼痛让他更加清醒。然后他从柜子里翻出一卷医用绷带，熟练地缠绕起左手的断指处，又用新的绷带蒙住左眼。左手断指处生长出细密的肉芽。".to_string();
        let chapters = vec![chapter];
        let anchors = anchors_for_test(&chapters);
        let bandage_anchor_id = anchor_id_containing(&anchors, "新的绷带蒙住左眼");
        let content = serde_json::json!({
            "objects": [
                {
                    "name": "绷带",
                    "aliases": [],
                    "object_type": "tool",
                    "importance": "secondary",
                    "visual_description": "用于包扎左眼和左手伤处的医用绷带",
                    "material_and_state": "新的医用绷带",
                    "story_usage": "遮盖伤口并维持角色伤势视觉连续性",
                    "visual_usage": "绑定左眼和左手伤处的包扎状态",
                    "generation_usage": "约束医用绷带外观、缠绕位置和遮盖关系",
                    "location_context": null,
                    "evidence_anchor_ids": [bandage_anchor_id]
                }
            ]
        })
        .to_string();
        let analyzed = BTreeSet::from([3]);

        let drafts =
            parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
                .expect("anchor id evidence should parse");

        assert_eq!(drafts[0].draft.evidence[0], "又用新的绷带蒙住左眼");
    }

    #[test]
    fn derives_object_chapter_scope_from_evidence_anchor_ids() {
        let mut later_chapter = chapter_for_test();
        later_chapter.id = 2;
        later_chapter.chapter_number = 4;
        later_chapter.content = "第四章里，那枚青铜钥匙被放在石门凹槽前。".to_string();
        let chapters = vec![chapter_for_test(), later_chapter];
        let anchors = anchors_for_test(&chapters);
        let first_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");
        let later_anchor_id = anchor_id_containing(&anchors, "那枚青铜钥匙");
        let content = serde_json::json!({
            "objects": [
                {
                    "name": "青铜钥匙",
                    "aliases": [],
                    "object_type": "token",
                    "importance": "primary",
                    "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                    "material_and_state": "青铜材质",
                    "story_usage": "开启后续密室的关键线索",
                    "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                    "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                    "location_context": "旧书房抽屉与石门凹槽前",
                    "evidence_anchor_ids": [first_anchor_id, later_anchor_id]
                }
            ]
        })
        .to_string();
        let analyzed = BTreeSet::from([3, 4]);

        let drafts =
            parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
                .expect("object chapter scope should be derived from evidence anchors");

        assert_eq!(drafts[0].draft.first_seen_chapter, 3);
        assert_eq!(drafts[0].draft.last_seen_chapter, 4);
    }

    #[test]
    fn rejects_model_supplied_object_chapter_scope_fields() {
        let chapters = vec![chapter_for_test()];
        let anchors = anchors_for_test(&chapters);
        let key_anchor_id = anchor_id_containing(&anchors, "一枚青铜钥匙");
        let content = serde_json::json!({
            "objects": [
                {
                    "name": "青铜钥匙",
                    "aliases": [],
                    "object_type": "token",
                    "importance": "primary",
                    "visual_description": "柄部刻着残缺星纹的青铜钥匙",
                    "material_and_state": "青铜材质",
                    "story_usage": "开启后续密室的关键线索",
                    "visual_usage": "保持青铜钥匙作为手中线索物的画面连续性",
                    "generation_usage": "约束青铜钥匙星纹、磨损边缘和青铜材质",
                    "location_context": null,
                    "first_seen_chapter": 3,
                    "evidence_anchor_ids": [key_anchor_id]
                }
            ]
        })
        .to_string();
        let analyzed = BTreeSet::from([3]);

        let err =
            parse_and_validate_object_discovery_content(&content, &analyzed, &chapters, &anchors)
                .expect_err("model supplied derived chapter field should fail");

        assert!(err.contains("未支持字段 `first_seen_chapter`"));
    }
}
