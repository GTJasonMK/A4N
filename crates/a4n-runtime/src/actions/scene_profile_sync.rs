use crate::actions::json_contract::{
    ensure_object, optional_positive_i64, optional_string, positive_i64_array,
    reject_unknown_fields, required_array, required_array_value, required_object_value,
    required_string, string_array, string_array_value,
};
use crate::actions::text_anchor_contract::{
    anchor_quotes_in_exact_chapter, reject_deprecated_reference_fields,
};
use crate::actions::{
    access,
    llm_request::PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT,
    llm_request::{ContextAwareJsonLlmRequestInput, context_aware_system_user_json_llm_request},
};
use crate::state::AppState;
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::models::asset::object_profile;
use a4n_core::models::chapter::chapter;
use a4n_core::models::character::protagonist_profile;
use a4n_core::models::project::text_anchor;
use a4n_core::models::scene::scene_profile;
use a4n_engine::llm::provider::ChatRequest;
use a4n_engine::prompt::request::build_json_task_user_message;
use a4n_services::generation::{
    RecordCompletedGenerationRunInput, context_aware_chat_request_trace_json,
};
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use a4n_services::profile_text::normalize_importance_value;
use a4n_services::scene::{
    SCENE_ENTITY_STATE_KEYS, SCENE_ENTITY_TYPES, SCENE_TRANSITION_MOVEMENT_TYPES, SCENE_TYPES,
    SceneEntityStateDraft, SceneOccurrenceDraft, SceneProfileDraft, SceneSyncInput,
    SceneTransitionDraft, normalize_movement_type_value, normalize_scene_entity_state_key_value,
    normalize_scene_entity_type_value, normalize_scene_type_value,
};
use a4n_services::structured_llm_stream::{
    ChatJsonStreamInput, StructuredLlmProgress, chat_json_stream_with_progress,
};
use a4n_services::structured_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
use a4n_services::text_anchor::{
    EVIDENCE_ROLE_FIRST_SEEN, PROFILE_KIND_SCENE, ProfileEvidenceInput, text_anchor_prompt_values,
};
use a4n_services::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tokio_util::sync::CancellationToken;

mod workflow;

use workflow::{
    finalize_scene_profile_sync_workflow, load_completed_scene_final_checkpoint,
    merge_scene_final_checkpoint, save_scene_final_checkpoint, scene_profile_sync_business_key,
    scene_profile_sync_workflow_fingerprint,
};

const SCENE_PROFILE_SYNC_PROGRESS_TOTAL: u32 = 100;
const SCENE_PROFILE_SYNC_CHAPTER_START: u32 = 5;
const SCENE_PROFILE_SYNC_CHAPTER_END: u32 = 88;
const SCENE_PROFILE_SYNC_RAG_PROGRESS: u32 = 94;
const SCENE_PROFILE_CANDIDATES_PROMPT: &str = "scene_profile_candidates";
const SCENE_OCCURRENCE_PROMPT: &str = "scene_occurrence_extraction";
const SCENE_TRANSITION_PROMPT: &str = "scene_transition_extraction";
const SCENE_ENTITY_STATE_PROMPT: &str = "scene_entity_state_extraction";
const SCENE_TYPE_RULE: &str = "scene_type 只能是 room/building/street/city/wilderness/route/landmark/other；走廊、通道、楼梯、电梯、道路、小路等连接路径使用 route，不要输出 corridor/hallway/road";
const SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE: &str = "scenes 只能包含本章正文中实际出现、进入、停留或发生行动的场景；只被提到、传闻、背景设定、心理想象或未实际出场的地点不要输出；每个输出的 scene_ref 后续必须能生成至少一条 occurrences";
const SCENE_OCCURRENCE_COVERAGE_RULE: &str = "必须覆盖 input.previous_stages.scenes[] 中的每一个 scene_ref：每个 scene_ref 至少输出一条 occurrence；只有 input.previous_stages.scenes 为空时才允许返回 occurrences: []";
const SCENE_TRANSITION_MOVEMENT_TYPE_RULE: &str = "movement_type 只能是 enter/leave/travel/chase/teleport/hidden_passage/cutaway；普通场景间移动或换地点使用 travel，不要输出 move";
const SCENE_ENTITY_TYPE_RULE: &str = "entity_type 只能是 character/object/event；物体、道具、装备或线索物统一使用 object，不要输出 item/prop/tool";
const SCENE_ENTITY_STATE_KEY_RULE: &str = "state_key 只能是 position/possession/condition/action/relation/event_progress；位置用 position，持有关系用 possession，状态/情绪/身体状况用 condition，动作行为用 action，关系用 relation，事件进展用 event_progress";
const SCENE_CANDIDATE_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("scene_ref", "场景引用", 1),
    FieldMilestone::new("name", "场景名称", 1),
    FieldMilestone::new("scene_type", "场景类型", 1),
    FieldMilestone::new("importance", "重要性", 1),
    FieldMilestone::new("base_description", "基础描述", 1),
    FieldMilestone::new("visual_description", "视觉描述", 1),
    FieldMilestone::new("spatial_layout", "空间布局", 1),
    FieldMilestone::new("atmosphere", "氛围", 1),
    FieldMilestone::new("evidence_anchor_ids", "证据锚点", 1),
];
const SCENE_OCCURRENCE_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("occurrence_ref", "出场引用", 1),
    FieldMilestone::new("scene_ref", "关联场景", 1),
    FieldMilestone::new("summary", "出场摘要", 1),
    FieldMilestone::new("characters", "出场角色", 1),
    FieldMilestone::new("objects", "出场物体", 1),
    FieldMilestone::new("entry_state", "进入状态", 1),
    FieldMilestone::new("exit_state", "离开状态", 1),
    FieldMilestone::new("source_anchor_ids", "来源锚点", 1),
];
const SCENE_TRANSITION_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("from_scene_ref", "起点场景", 1),
    FieldMilestone::new("to_scene_ref", "目标场景", 1),
    FieldMilestone::new("movement_type", "移动类型", 1),
    FieldMilestone::new("route_description", "路线描述", 1),
    FieldMilestone::new("participants", "参与角色", 1),
    FieldMilestone::new("object_refs", "关联物体", 1),
    FieldMilestone::new("source_anchor_ids", "来源锚点", 1),
];
const SCENE_ENTITY_STATE_PROGRESS_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("scene_ref", "关联场景", 1),
    FieldMilestone::new("occurrence_ref", "关联出场", 1),
    FieldMilestone::new("entity_type", "实体类型", 1),
    FieldMilestone::new("entity_id", "实体 ID", 1),
    FieldMilestone::new("state_key", "状态类型", 1),
    FieldMilestone::new("state_before", "变化前状态", 1),
    FieldMilestone::new("state_after", "变化后状态", 1),
    FieldMilestone::new("action_description", "动作描述", 1),
    FieldMilestone::new("source_anchor_ids", "来源锚点", 1),
];
pub type SceneProfileSyncProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

pub struct SceneProfileSyncProgress<'a> {
    reporter: Option<SceneProfileSyncProgressReporter<'a>>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> SceneProfileSyncProgress<'a> {
    pub fn new(
        reporter: Option<SceneProfileSyncProgressReporter<'a>>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneProfileSyncTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct SceneProfileSyncPreviewStep {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) prompt_name: Option<String>,
    pub(crate) state: String,
    pub(crate) note: Option<String>,
    pub(crate) request: Option<ChatRequest>,
}

struct SceneProfileSyncRequestContext {
    chapter: chapter::Model,
    text_anchors: Vec<text_anchor::Model>,
    existing_scene_profiles: Vec<scene_profile::Model>,
    character_profiles: Vec<protagonist_profile::Model>,
    object_profiles: Vec<object_profile::Model>,
    existing_scene_ids: BTreeSet<i64>,
    character_ids: BTreeSet<i64>,
    object_ids: BTreeSet<i64>,
    character_names: BTreeMap<i64, String>,
    object_names: BTreeMap<i64, String>,
}

#[derive(Debug, Clone, Copy)]
enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

struct SceneProfileSyncStageResponses {
    scenes: Value,
    occurrences: Value,
    transitions: Value,
    entity_states: Value,
    requests: Vec<ChatRequest>,
    response_chars: Vec<Value>,
}

#[derive(Debug, Clone, Copy)]
struct SceneProfileStageProgressSpec {
    structured: StructuredJsonProgressSpec,
    span: ProgressSpan,
}

struct SceneProfileStageCallResult {
    output: Value,
    response_content: String,
}

struct SceneProfileStageCheckpoint {
    response_content: String,
    parsed_output: Value,
}

#[derive(Debug)]
struct ParsedSceneProfileSync {
    input: SceneSyncInput,
    profile_evidence: Vec<SceneProfileEvidenceRefs>,
}

#[derive(Debug)]
struct SceneProfileEvidenceRefs {
    scene_ref: String,
    evidence_anchor_ids: Vec<i64>,
}

pub async fn sync_scene_profiles_for_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    progress: Option<&SceneProfileSyncProgress<'_>>,
) -> Result<Value, String> {
    scene_profile_sync_task_scope(state, user_id, project_id).await?;
    report_scene_profile_sync_progress(progress, 0, "正在读取可同步章节", "场景档案同步已停止")?;
    let chapters = select_scene_profile_sync_chapters(state, project_id, chapter_number).await?;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: scene_profile_sync_business_key(project_id, chapter_number),
            workflow_kind: "scene_profile_sync".to_string(),
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
            input_fingerprint_json: Some(scene_profile_sync_workflow_fingerprint(
                project_id,
                chapter_number,
                &chapters,
            )),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("scene_profile_sync.prepare".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let mut profile_ids = BTreeSet::new();
        let mut occurrence_count = 0usize;
        let mut transition_count = 0usize;
        let mut entity_state_count = 0usize;
        let mut profile_summaries = BTreeMap::new();

        let chapter_count = chapters.len();
        let chapter_progress_span = ProgressSpan::new(
            SCENE_PROFILE_SYNC_CHAPTER_START,
            SCENE_PROFILE_SYNC_CHAPTER_END,
        );
        for (chapter_index, chapter) in chapters.into_iter().enumerate() {
            let context = build_scene_profile_sync_request(
                state,
                project_id,
                chapter,
                ChapterAnchorMode::EnsureCurrent,
            )
            .await?;
            let chapter_span = chapter_progress_span.split(chapter_index, chapter_count);
            report_scene_profile_sync_progress(
                progress,
                chapter_span.start(),
                format!(
                    "正在分阶段同步第 {} 章场景档案",
                    context.chapter.chapter_number
                ),
                "场景档案同步已停止",
            )?;
            if let Some(checkpoint) = load_completed_scene_final_checkpoint(
                state,
                workflow_run_id,
                &context,
                chapter_index,
            )
            .await?
            {
                merge_scene_final_checkpoint(
                    checkpoint,
                    &mut profile_ids,
                    &mut occurrence_count,
                    &mut transition_count,
                    &mut entity_state_count,
                    &mut profile_summaries,
                )?;
                report_scene_profile_sync_progress(
                    progress,
                    subspan(chapter_span, 5, 6, 6).end(),
                    format!(
                        "第 {} 章场景档案已从断点恢复",
                        context.chapter.chapter_number
                    ),
                    "场景档案同步已停止",
                )?;
                continue;
            }
            let stage_responses = run_scene_profile_sync_stages(
                state,
                workflow_run_id,
                &context,
                progress,
                chapter_span,
                chapter_index,
            )
            .await?;
            report_scene_profile_sync_progress(
                progress,
                subspan(chapter_span, 4, 5, 6).start(),
                format!(
                    "第 {} 章模型已返回，正在校验场景档案结果",
                    context.chapter.chapter_number
                ),
                "场景档案同步已停止",
            )?;
            let response_content = serde_json::to_string(&json!({
                "scenes": stage_responses.scenes,
                "occurrences": stage_responses.occurrences,
                "transitions": stage_responses.transitions,
                "entity_states": stage_responses.entity_states,
            }))
            .map_err(|err| format!("场景档案同步阶段结果序列化失败: {err}"))?;
            let parsed = parse_scene_profile_sync_content(&response_content, &context)?;
            report_scene_profile_sync_progress(
                progress,
                subspan(chapter_span, 5, 6, 6).start(),
                format!(
                    "正在写入第 {} 章场景档案、出场和场景状态",
                    context.chapter.chapter_number
                ),
                "场景档案同步已停止",
            )?;
            let request_json = stage_responses
                .requests
                .iter()
                .map(context_aware_chat_request_trace_json)
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
                .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
            let messages_json = serde_json::to_value(
                stage_responses
                    .requests
                    .iter()
                    .map(|request| request.messages.clone())
                    .collect::<Vec<_>>(),
            )
            .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
            let result = state
                .scene_profile
                .replace_chapter_scene_sync_with_generation_run(
                    project_id,
                    context.chapter.id,
                    context.chapter.chapter_number,
                    parsed.input,
                    RecordCompletedGenerationRunInput {
                        project_id,
                        workflow: "scene_profile_sync".to_string(),
                        step_key: "sync_scene_profiles_for_chapter".to_string(),
                        title: "场景档案同步".to_string(),
                        model_kind: "llm".to_string(),
                        provider: None,
                        model: None,
                        prompt_template_names_json: Some(json!([
                            SCENE_PROFILE_CANDIDATES_PROMPT,
                            SCENE_OCCURRENCE_PROMPT,
                            SCENE_TRANSITION_PROMPT,
                            SCENE_ENTITY_STATE_PROMPT
                        ])),
                        prompt_template_hashes_json: None,
                        rendered_messages_json: Some(messages_json),
                        rendered_prompts_json: None,
                        input_variables_json: Some(json!({
                            "chapter_id": context.chapter.id,
                            "chapter_number": context.chapter.chapter_number,
                            "text_anchor_count": context.text_anchors.len(),
                            "existing_scene_ids": &context.existing_scene_ids,
                            "character_ids": &context.character_ids,
                            "object_ids": &context.object_ids,
                        })),
                        source_refs_json: Some(json!({
                            "chapter": {
                                "id": context.chapter.id,
                                "chapter_number": context.chapter.chapter_number,
                                "title": &context.chapter.title,
                            },
                        })),
                        media_refs_json: None,
                        request_json: Some(request_json),
                        response_summary_json: Some(json!({
                            "stages": stage_responses.response_chars,
                            "assembled_response_chars": response_content.chars().count(),
                        })),
                        targets: Vec::new(),
                    },
                )
                .await
                .map_err(|err| err.to_string())?;
            bind_scene_profile_evidence(state, project_id, &parsed.profile_evidence, &result)
                .await?;
            occurrence_count += result.occurrence_count;
            transition_count += result.transition_count;
            entity_state_count += result.entity_state_count;
            for id in &result.profile_ids {
                if profile_ids.insert(*id) {
                    let profile = state
                        .scene_profile
                        .get(*id)
                        .await
                        .map_err(|err| err.to_string())?;
                    profile_summaries.insert(*id, scene_profile_result(&profile));
                }
            }
            save_scene_final_checkpoint(
                state,
                workflow_run_id,
                &context,
                chapter_index,
                &result,
                &profile_summaries,
            )
            .await?;
        }

        report_scene_profile_sync_progress(
            progress,
            SCENE_PROFILE_SYNC_RAG_PROGRESS,
            "正在同步场景档案 RAG",
            "场景档案同步已停止",
        )?;
        let rag = crate::rag_ingest::sync_scene_rag_records(state, project_id).await?;

        report_scene_profile_sync_progress(
            progress,
            SCENE_PROFILE_SYNC_PROGRESS_TOTAL,
            "场景档案同步已完成",
            "场景档案同步已停止",
        )?;
        Ok(json!({
            "status": "synced",
            "profile_ids": profile_ids.iter().copied().collect::<Vec<_>>(),
            "profiles": profile_summaries.into_values().collect::<Vec<_>>(),
            "occurrence_count": occurrence_count,
            "transition_count": transition_count,
            "entity_state_count": entity_state_count,
            "rag": rag,
        }))
    }
    .await;

    if let Err(status_err) =
        finalize_scene_profile_sync_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "场景档案同步完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

pub async fn scene_profile_sync_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<SceneProfileSyncTaskScope, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    Ok(SceneProfileSyncTaskScope {
        project_id,
        total: SCENE_PROFILE_SYNC_PROGRESS_TOTAL,
    })
}

pub(crate) async fn scene_profile_sync_preview_steps(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    ready_state: &str,
    ready_note: Option<String>,
) -> Result<Vec<SceneProfileSyncPreviewStep>, String> {
    scene_profile_sync_task_scope(state, user_id, project_id).await?;
    let chapters = select_scene_profile_sync_chapters(state, project_id, chapter_number).await?;
    let mut steps = Vec::new();
    for chapter in chapters {
        let context = build_scene_profile_sync_request(
            state,
            project_id,
            chapter,
            ChapterAnchorMode::RequireExistingCurrent,
        )
        .await?;
        let chapter_number = context.chapter.chapter_number;
        let scene_request = build_scene_profile_stage_request(
            state,
            &context,
            None,
            SCENE_PROFILE_CANDIDATES_PROMPT,
            "识别或匹配当前章节中的可复用场景档案，只输出 scenes",
            json!({}),
            json!({
                "format": "json_object",
                "required_root_field": "scenes",
                "strict_rules": [
                    "只返回 scenes 数组，不要输出 occurrences、transitions 或 entity_states",
                    SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE,
                    SCENE_TYPE_RULE,
                    "importance 只能是 primary/secondary/minor",
                    "evidence_anchor_ids 必须来自 input.text_anchors[].id",
                    "scene_profile_id 和 parent_scene_id 只能引用 input.existing_scene_profiles[].id 或 null"
                ]
            }),
        )?;
        steps.push(SceneProfileSyncPreviewStep {
            key: format!("scene_profile_sync_ch{chapter_number}_scenes"),
            title: format!("第 {chapter_number} 章场景档案候选识别请求"),
            prompt_name: Some(SCENE_PROFILE_CANDIDATES_PROMPT.to_string()),
            state: ready_state.to_string(),
            note: ready_note.clone(),
            request: Some(scene_request),
        });
        steps.extend(blocked_scene_profile_dependent_preview_steps(
            chapter_number,
        ));
    }
    Ok(steps)
}

fn blocked_scene_profile_dependent_preview_steps(
    chapter_number: i32,
) -> Vec<SceneProfileSyncPreviewStep> {
    vec![
        SceneProfileSyncPreviewStep {
            key: format!("scene_profile_sync_ch{chapter_number}_occurrences"),
            title: format!("第 {chapter_number} 章场景出场抽取请求"),
            prompt_name: Some(SCENE_OCCURRENCE_PROMPT.to_string()),
            state: "blocked".to_string(),
            note: Some(
                "该请求依赖场景档案候选识别返回的 scenes，发送前无法提前构造真实请求".to_string(),
            ),
            request: None,
        },
        SceneProfileSyncPreviewStep {
            key: format!("scene_profile_sync_ch{chapter_number}_transitions"),
            title: format!("第 {chapter_number} 章场景转场抽取请求"),
            prompt_name: Some(SCENE_TRANSITION_PROMPT.to_string()),
            state: "blocked".to_string(),
            note: Some(
                "该请求依赖场景档案候选和场景出场模型返回结果，发送前无法提前构造真实请求"
                    .to_string(),
            ),
            request: None,
        },
        SceneProfileSyncPreviewStep {
            key: format!("scene_profile_sync_ch{chapter_number}_entity_states"),
            title: format!("第 {chapter_number} 章场景实体状态抽取请求"),
            prompt_name: Some(SCENE_ENTITY_STATE_PROMPT.to_string()),
            state: "blocked".to_string(),
            note: Some(
                "该请求依赖场景档案候选和场景出场模型返回结果，发送前无法提前构造真实请求"
                    .to_string(),
            ),
            request: None,
        },
    ]
}

async fn build_scene_profile_sync_request(
    state: &AppState,
    project_id: i64,
    chapter: chapter::Model,
    anchor_mode: ChapterAnchorMode,
) -> Result<SceneProfileSyncRequestContext, String> {
    let existing_scene_profiles = state
        .scene_profile
        .list_by_project(project_id)
        .await
        .map_err(|err| err.to_string())?;
    let existing_scene_ids = existing_scene_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<BTreeSet<_>>();
    let object_profiles = state
        .object_profile
        .list_by_project(project_id)
        .await
        .map_err(|err| err.to_string())?;
    let object_ids = object_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<BTreeSet<_>>();
    let object_names = object_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<BTreeMap<_, _>>();
    let character_profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let character_ids = character_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<BTreeSet<_>>();
    let character_names = character_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<BTreeMap<_, _>>();

    let text_anchors = match anchor_mode {
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
    Ok(SceneProfileSyncRequestContext {
        chapter,
        text_anchors,
        existing_scene_profiles,
        character_profiles,
        object_profiles,
        existing_scene_ids,
        character_ids,
        object_ids,
        character_names,
        object_names,
    })
}

async fn select_scene_profile_sync_chapters(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, String> {
    if project_id <= 0 {
        return Err("场景档案同步 project_id 必须大于 0".to_string());
    }
    if chapter_number.is_some_and(|value| value <= 0) {
        return Err("场景档案同步章节号必须大于 0".to_string());
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
                "第 {chapter_number} 章没有可用于场景档案同步的有效正文"
            ));
        }
        return Ok(chapters);
    }
    if chapters.is_empty() {
        return Err(format!(
            "项目还没有可用于场景档案同步的章节: project_id={project_id}"
        ));
    }
    Ok(chapters)
}

fn existing_scene_profile_for_prompt(profile: &scene_profile::Model) -> Value {
    json!({
        "id": profile.id,
        "name": profile.name,
        "aliases": profile.aliases_json,
        "scene_type": profile.scene_type,
        "importance": profile.importance,
        "base_description": profile.base_description,
        "visual_description": profile.visual_description,
        "spatial_layout": profile.spatial_layout,
        "atmosphere": profile.atmosphere,
        "current_state": profile.current_state_json,
        "first_seen_chapter": profile.first_seen_chapter,
        "last_seen_chapter": profile.last_seen_chapter,
    })
}

fn character_profile_for_prompt(profile: &protagonist_profile::Model) -> Value {
    json!({
        "profile_id": profile.id,
        "name": profile.name,
        "appearance": profile.appearance,
        "current_state": profile.current_state,
        "last_synced_chapter": profile.last_synced_chapter,
    })
}

fn object_profile_for_prompt(profile: &object_profile::Model) -> Value {
    json!({
        "object_profile_id": profile.id,
        "name": profile.name,
        "object_type": profile.object_type,
        "visual_description": profile.visual_description,
        "material_and_state": profile.material_and_state,
        "location_context": profile.location_context,
        "last_seen_chapter": profile.last_seen_chapter,
    })
}

fn scene_profile_chapter_context(chapter: &chapter::Model) -> Value {
    json!({
        "chapter_id": chapter.id,
        "chapter_number": chapter.chapter_number,
        "chapter_title": chapter.title,
        "content": chapter.content,
        "analysis_data": chapter.analysis_data,
    })
}

fn subspan(span: ProgressSpan, start_units: u32, end_units: u32, total_units: u32) -> ProgressSpan {
    let total_units = total_units.max(1);
    let start_units = start_units.min(total_units);
    let end_units = end_units.min(total_units).max(start_units);
    let width = span.end().saturating_sub(span.start());
    ProgressSpan::new(
        span.start() + width.saturating_mul(start_units) / total_units,
        span.start() + width.saturating_mul(end_units) / total_units,
    )
}

fn scene_stage_progress_spec(
    root_array_key: &'static str,
    item_label: &'static str,
    milestones: &'static [FieldMilestone],
    span: ProgressSpan,
) -> SceneProfileStageProgressSpec {
    SceneProfileStageProgressSpec {
        structured: StructuredJsonProgressSpec {
            root_array_key,
            item_label,
            expected_items: None,
            milestones,
        },
        span,
    }
}

async fn run_scene_profile_sync_stages(
    state: &AppState,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    chapter_span: ProgressSpan,
    chapter_index: usize,
) -> Result<SceneProfileSyncStageResponses, String> {
    let mut requests = Vec::new();
    let mut response_chars = Vec::new();
    let model_span = subspan(chapter_span, 0, 4, 6);

    let scene_request = build_scene_profile_stage_request(
        state,
        context,
        Some(workflow_run_id),
        SCENE_PROFILE_CANDIDATES_PROMPT,
        "识别或匹配当前章节中的可复用场景档案，只输出 scenes",
        json!({}),
        json!({
            "format": "json_object",
            "required_root_field": "scenes",
            "strict_rules": [
                "只返回 scenes 数组，不要输出 occurrences、transitions 或 entity_states",
                SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE,
                SCENE_TYPE_RULE,
                "importance 只能是 primary/secondary/minor",
                "evidence_anchor_ids 必须来自 input.text_anchors[].id",
                "scene_profile_id 和 parent_scene_id 只能引用 input.existing_scene_profiles[].id 或 null"
            ]
        }),
    )?;
    let scenes = run_scene_profile_stage_with_checkpoint(
        state,
        workflow_run_id,
        scene_request.clone(),
        SCENE_PROFILE_CANDIDATES_PROMPT,
        "scenes",
        scene_profile_stage_step_key(context, "scenes"),
        scene_profile_stage_step_order(chapter_index, 0),
        scene_stage_progress_spec(
            "scenes",
            "场景",
            SCENE_CANDIDATE_PROGRESS_FIELDS,
            model_span.split(0, 4),
        ),
        progress,
        &mut response_chars,
    )
    .await?;
    requests.push(scene_request);

    let occurrence_request = build_scene_profile_stage_request(
        state,
        context,
        Some(workflow_run_id),
        SCENE_OCCURRENCE_PROMPT,
        "基于已确认场景抽取当前章节中的场景出场，只输出 occurrences",
        json!({ "scenes": scenes.clone() }),
        json!({
            "format": "json_object",
            "required_root_field": "occurrences",
            "strict_rules": [
                "只返回 occurrences 数组，不要输出 scenes、transitions 或 entity_states",
                "scene_ref 必须来自 input.previous_stages.scenes[].scene_ref",
                SCENE_OCCURRENCE_COVERAGE_RULE,
                "source_anchor_ids 必须来自 input.text_anchors[].id"
            ]
        }),
    )?;
    let occurrences = run_scene_profile_stage_with_checkpoint(
        state,
        workflow_run_id,
        occurrence_request.clone(),
        SCENE_OCCURRENCE_PROMPT,
        "occurrences",
        scene_profile_stage_step_key(context, "occurrences"),
        scene_profile_stage_step_order(chapter_index, 1),
        scene_stage_progress_spec(
            "occurrences",
            "场景出场",
            SCENE_OCCURRENCE_PROGRESS_FIELDS,
            model_span.split(1, 4),
        ),
        progress,
        &mut response_chars,
    )
    .await?;
    requests.push(occurrence_request);

    let transition_request = build_scene_profile_stage_request(
        state,
        context,
        Some(workflow_run_id),
        SCENE_TRANSITION_PROMPT,
        "基于场景与出场链抽取场景间转场，只输出 transitions",
        json!({
            "scenes": scenes.clone(),
            "occurrences": occurrences.clone(),
        }),
        json!({
            "format": "json_object",
            "required_root_field": "transitions",
            "strict_rules": [
                "只返回 transitions 数组，不要输出 scenes、occurrences 或 entity_states",
                "to_scene_ref/from_scene_ref 必须来自 input.previous_stages.scenes[].scene_ref 或 null",
                "occurrence_ref 必须来自 input.previous_stages.occurrences[].occurrence_ref 或 null",
                SCENE_TRANSITION_MOVEMENT_TYPE_RULE,
                "source_anchor_ids 必须来自 input.text_anchors[].id"
            ]
        }),
    )?;
    let transitions = run_scene_profile_stage_with_checkpoint(
        state,
        workflow_run_id,
        transition_request.clone(),
        SCENE_TRANSITION_PROMPT,
        "transitions",
        scene_profile_stage_step_key(context, "transitions"),
        scene_profile_stage_step_order(chapter_index, 2),
        scene_stage_progress_spec(
            "transitions",
            "场景转场",
            SCENE_TRANSITION_PROGRESS_FIELDS,
            model_span.split(2, 4),
        ),
        progress,
        &mut response_chars,
    )
    .await?;
    requests.push(transition_request);

    let entity_request = build_scene_profile_stage_request(
        state,
        context,
        Some(workflow_run_id),
        SCENE_ENTITY_STATE_PROMPT,
        "基于场景与出场链抽取场景内实体状态，只输出 entity_states",
        json!({
            "scenes": scenes.clone(),
            "occurrences": occurrences.clone(),
        }),
        json!({
            "format": "json_object",
            "required_root_field": "entity_states",
            "strict_rules": [
                "只返回 entity_states 数组，不要输出 scenes、occurrences 或 transitions",
                "scene_ref 必须来自 input.previous_stages.scenes[].scene_ref",
                "occurrence_ref 必须来自 input.previous_stages.occurrences[].occurrence_ref",
                SCENE_ENTITY_TYPE_RULE,
                SCENE_ENTITY_STATE_KEY_RULE,
                "实体显示名称由系统根据 entity_id 或动作描述生成，模型只需输出实体类型、ID 和状态变化",
                "source_anchor_ids 必须来自 input.text_anchors[].id"
            ]
        }),
    )?;
    let entity_states = run_scene_profile_stage_with_checkpoint(
        state,
        workflow_run_id,
        entity_request.clone(),
        SCENE_ENTITY_STATE_PROMPT,
        "entity_states",
        scene_profile_stage_step_key(context, "entity_states"),
        scene_profile_stage_step_order(chapter_index, 3),
        scene_stage_progress_spec(
            "entity_states",
            "场景实体状态",
            SCENE_ENTITY_STATE_PROGRESS_FIELDS,
            model_span.split(3, 4),
        ),
        progress,
        &mut response_chars,
    )
    .await?;
    requests.push(entity_request);

    Ok(SceneProfileSyncStageResponses {
        scenes,
        occurrences,
        transitions,
        entity_states,
        requests,
        response_chars,
    })
}

fn build_scene_profile_stage_request(
    state: &AppState,
    context: &SceneProfileSyncRequestContext,
    workflow_run_id: Option<i64>,
    prompt_name: &str,
    instruction: &str,
    previous_stages: Value,
    output_contract: Value,
) -> Result<ChatRequest, String> {
    let system_prompt = state
        .engine
        .render(
            prompt_name,
            &HashMap::new(),
            Some(context.chapter.project_id),
        )
        .map_err(|err| err.to_string())?;
    let payload = json!({
        "task": {
            "name": prompt_name,
            "instruction": instruction
        },
        "output_contract": output_contract,
        "input": {
            "context_refs": {
                "existing_scene_profiles": format!("稳定上下文包 {PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT}.existing_scene_profiles"),
                "available_characters": format!("稳定上下文包 {PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT}.available_characters"),
                "available_object_profiles": format!("稳定上下文包 {PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT}.available_object_profiles"),
                "chapter": format!("稳定上下文包 {PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT}.chapter"),
                "text_anchors": format!("稳定上下文包 {PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT}.text_anchors")
            },
            "previous_stages": previous_stages
        }
    });
    let user_prompt = build_json_task_user_message(
        "执行场景档案同步阶段",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|err| format!("场景档案同步阶段上下文序列化失败: {err}"))?;
    context_aware_system_user_json_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id: context.chapter.project_id,
            workflow_run_id,
            workflow: "scene_profile_sync".to_string(),
            step_key: scene_profile_stage_step_key(context, prompt_name),
            scope_kind: "chapter".to_string(),
            pack_kind: PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "chapter_id": context.chapter.id,
                "chapter_number": context.chapter.chapter_number,
                "prompt_name": prompt_name,
                "text_anchor_count": context.text_anchors.len(),
                "existing_scene_count": context.existing_scene_profiles.len(),
                "character_count": context.character_profiles.len(),
                "object_count": context.object_profiles.len(),
            })),
            config_json: json!({
                "ledger_version": "scene_profile_sync_context_v1",
                "prompt_name": prompt_name,
                "chapter_id": context.chapter.id,
                "chapter_number": context.chapter.chapter_number,
            }),
            stable_context_json: scene_profile_stage_stable_context(context),
            dynamic_user_prompt: user_prompt,
            temperature: Some(0.2),
            max_tokens_profile: llm_token_profiles::CHARACTER_DISCOVERY.to_string(),
            call_key: Some(format!(
                "scene_profile_sync:{}:{}",
                context.chapter.id, prompt_name
            )),
        },
    )
}

fn scene_profile_stage_stable_context(context: &SceneProfileSyncRequestContext) -> Value {
    json!({
        "existing_scene_profiles": context.existing_scene_profiles.iter().map(existing_scene_profile_for_prompt).collect::<Vec<_>>(),
        "available_characters": context.character_profiles.iter().map(character_profile_for_prompt).collect::<Vec<_>>(),
        "available_object_profiles": context.object_profiles.iter().map(object_profile_for_prompt).collect::<Vec<_>>(),
        "chapter": scene_profile_chapter_context(&context.chapter),
        "text_anchors": text_anchor_prompt_values(std::slice::from_ref(&context.chapter), &context.text_anchors),
    })
}

fn scene_profile_stage_step_key(context: &SceneProfileSyncRequestContext, stage: &str) -> String {
    format!(
        "scene_profile_sync.chapter.{}.{}",
        context.chapter.chapter_number, stage
    )
}

fn scene_profile_stage_step_order(chapter_index: usize, stage_index: usize) -> i32 {
    i32::try_from(
        chapter_index
            .saturating_mul(10)
            .saturating_add(stage_index)
            .saturating_add(1),
    )
    .unwrap_or(i32::MAX)
}

fn scene_profile_stage_fingerprint(
    call_key: &str,
    prompt_name: &str,
    root_field: &str,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "scene_profile_sync",
        "call_key": call_key,
        "prompt_name": prompt_name,
        "root_field": root_field,
        "request": context_aware_chat_request_trace_json(request)
            .map_err(|err| format!("场景档案同步阶段请求序列化失败: {err}"))?,
    }))
}

async fn run_scene_profile_stage_with_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    mut request: ChatRequest,
    prompt_name: &str,
    root_field: &str,
    step_key: String,
    step_order: i32,
    stage_progress: SceneProfileStageProgressSpec,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    response_chars: &mut Vec<Value>,
) -> Result<Value, String> {
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.call_key = Some(step_key.clone());
    }
    let fingerprint =
        scene_profile_stage_fingerprint(&step_key, prompt_name, root_field, &request)?;
    let fingerprint_sha =
        llm_call_checkpoint_fingerprint_sha256(&fingerprint).map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.input_fingerprint_sha256 = Some(fingerprint_sha);
    }
    if let Some(checkpoint) = load_completed_scene_profile_stage_checkpoint(
        state,
        workflow_run_id,
        &step_key,
        &fingerprint,
    )
    .await?
    {
        let output =
            parse_scene_stage_root_array(&checkpoint.response_content, prompt_name, root_field)?;
        let parsed_output = scene_profile_stage_parsed_output_json(root_field, &output);
        if parsed_output != checkpoint.parsed_output {
            return Err(format!(
                "{step_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
            ));
        }
        let item_count = output.as_array().map_or(0, Vec::len);
        response_chars.push(json!({
            "prompt_name": prompt_name,
            "response_chars": checkpoint.response_content.chars().count(),
            "source": "llm_call_checkpoint",
        }));
        report_scene_profile_sync_progress(
            progress,
            stage_progress.span.end(),
            format!("{prompt_name} 已从断点恢复：解析到 {item_count} 条{root_field}"),
            "场景档案同步已停止",
        )?;
        return Ok(output);
    }

    let call_result = chat_scene_profile_stage(
        state,
        request,
        prompt_name,
        root_field,
        stage_progress,
        progress,
        response_chars,
    )
    .await?;
    save_completed_scene_profile_stage_checkpoint(
        state,
        workflow_run_id,
        step_key.clone(),
        step_order,
        fingerprint,
        &call_result.response_content,
        scene_profile_stage_parsed_output_json(root_field, &call_result.output),
    )
    .await?;
    Ok(call_result.output)
}

async fn chat_scene_profile_stage(
    state: &AppState,
    request: ChatRequest,
    prompt_name: &str,
    root_field: &str,
    stage_progress: SceneProfileStageProgressSpec,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    response_chars: &mut Vec<Value>,
) -> Result<SceneProfileStageCallResult, String> {
    let truncated_message = format!(
        "{prompt_name} 输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“角色/档案发现”输出档位，或减少本次章节上下文后重试"
    );
    let content = chat_json_stream_with_progress(
        ChatJsonStreamInput {
            llm: state.llm.as_ref(),
            request,
            prompt_name,
            truncated_message: &truncated_message,
            structured_progress: Some(StructuredLlmProgress {
                spec: stage_progress.structured,
                span: stage_progress.span,
            }),
            cancel_token: progress.and_then(|progress| progress.cancel_token),
            cancelled_message: "场景档案同步已停止",
        },
        |current, message| {
            report_scene_profile_sync_progress(progress, current, message, "场景档案同步已停止")
        },
    )
    .await?;
    response_chars.push(json!({
        "prompt_name": prompt_name,
        "response_chars": content.chars().count(),
    }));
    let parsed = parse_scene_stage_root_array(&content, prompt_name, root_field)?;
    let item_count = parsed.as_array().map_or(0, Vec::len);
    report_scene_profile_sync_progress(
        progress,
        stage_progress.span.end(),
        format!("{prompt_name} 已完成：解析到 {item_count} 条{root_field}"),
        "场景档案同步已停止",
    )?;
    Ok(SceneProfileStageCallResult {
        output: parsed,
        response_content: content,
    })
}

fn scene_profile_stage_parsed_output_json(root_field: &str, output: &Value) -> Value {
    let mut object = serde_json::Map::new();
    object.insert(root_field.to_string(), output.clone());
    Value::Object(object)
}

async fn load_completed_scene_profile_stage_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<SceneProfileStageCheckpoint>, String> {
    let Some(checkpoint) = load_completed_llm_call_checkpoint(
        state.db.clone(),
        workflow_run_id,
        call_key,
        input_fingerprint_json,
    )
    .await
    .map_err(|err| err.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json =
        required_llm_call_checkpoint_json(checkpoint, call_key).map_err(|err| err.to_string())?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.response_content 必须是字符串"))?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
    Ok(Some(SceneProfileStageCheckpoint {
        response_content,
        parsed_output,
    }))
}

async fn save_completed_scene_profile_stage_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    call_key: String,
    call_order: i32,
    input_fingerprint_json: Value,
    response_content: &str,
    parsed_output: Value,
) -> Result<(), String> {
    save_completed_llm_call_checkpoint(
        state.db.clone(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order,
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

fn parse_scene_stage_root_array(
    content: &str,
    prompt_name: &str,
    root_field: &str,
) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        let mut message = format!("{prompt_name} 结果必须是 JSON 对象: {err}");
        if err.is_eof() {
            message.push_str("；响应在 JSON 闭合前结束，通常是模型输出被截断或 max_tokens 不足");
        }
        message
    })?;
    if !value.is_object() {
        return Err(format!("{prompt_name} 结果必须是 JSON 对象"));
    }
    reject_unknown_fields(&value, prompt_name, &[root_field])?;
    value
        .get(root_field)
        .and_then(Value::as_array)
        .cloned()
        .map(Value::Array)
        .ok_or_else(|| format!("{prompt_name}.{root_field} 必须是数组"))
}

fn parse_scene_profile_sync_content(
    content: &str,
    context: &SceneProfileSyncRequestContext,
) -> Result<ParsedSceneProfileSync, String> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        let mut message = format!("场景档案同步结果必须是 JSON 对象: {err}");
        if err.is_eof() {
            message.push_str("；响应在 JSON 闭合前结束，通常是模型输出被截断或 max_tokens 不足");
        }
        message
    })?;
    if !value.is_object() {
        return Err("场景档案同步结果必须是 JSON 对象".to_string());
    }
    reject_unknown_fields(
        &value,
        "root",
        &["scenes", "occurrences", "transitions", "entity_states"],
    )?;
    let profiles = parse_scene_profiles(&value, context)?;
    let input = SceneSyncInput {
        profiles: profiles.drafts,
        occurrences: parse_scene_occurrences(&value, context)?,
        transitions: parse_scene_transitions(&value, context)?,
        entity_states: parse_scene_entity_states(&value, context)?,
    };
    validate_scene_sync_references(&input)?;
    Ok(ParsedSceneProfileSync {
        input,
        profile_evidence: profiles.evidence_refs,
    })
}

struct ParsedSceneProfiles {
    drafts: Vec<SceneProfileDraft>,
    evidence_refs: Vec<SceneProfileEvidenceRefs>,
}

fn parse_scene_profiles(
    root: &Value,
    context: &SceneProfileSyncRequestContext,
) -> Result<ParsedSceneProfiles, String> {
    let scenes = required_array(root, "scenes", "root")?;
    let mut refs = BTreeSet::new();
    let mut drafts = Vec::with_capacity(scenes.len());
    let mut evidence_refs = Vec::with_capacity(scenes.len());
    for (index, scene) in scenes.iter().enumerate() {
        let path = format!("scenes[{index}]");
        ensure_object(scene, &path)?;
        reject_deprecated_evidence_fields(scene, &path, "evidence_anchor_ids")?;
        reject_unknown_fields(
            scene,
            &path,
            &[
                "scene_ref",
                "scene_profile_id",
                "name",
                "aliases",
                "scene_type",
                "importance",
                "parent_scene_id",
                "base_description",
                "visual_description",
                "spatial_layout",
                "atmosphere",
                "entry_points",
                "rules",
                "evidence_anchor_ids",
            ],
        )?;
        let scene_ref = required_string(scene, "scene_ref", &path)?;
        if !refs.insert(scene_ref.clone()) {
            return Err(format!("{path}.scene_ref 与前序场景重复: {scene_ref}"));
        }
        let scene_profile_id = optional_positive_i64(scene, "scene_profile_id", &path)?;
        if let Some(id) = scene_profile_id {
            validate_id_in_set(
                id,
                &context.existing_scene_ids,
                &format!("{path}.scene_profile_id"),
            )?;
        }
        let parent_scene_id = optional_positive_i64(scene, "parent_scene_id", &path)?;
        if let Some(id) = parent_scene_id {
            validate_id_in_set(
                id,
                &context.existing_scene_ids,
                &format!("{path}.parent_scene_id"),
            )?;
        }
        let first_seen_chapter = context.chapter.chapter_number;
        let last_seen_chapter = context.chapter.chapter_number;
        let evidence_anchor_ids = positive_i64_array(scene, "evidence_anchor_ids", &path, true)?;
        let evidence = resolve_scene_text_anchor_texts(
            &format!("{path}.evidence_anchor_ids"),
            &evidence_anchor_ids,
            context,
        )?;
        evidence_refs.push(SceneProfileEvidenceRefs {
            scene_ref: scene_ref.clone(),
            evidence_anchor_ids,
        });
        let name = required_string(scene, "name", &path)?;
        let base_description = required_string(scene, "base_description", &path)?;
        let visual_description = required_string(scene, "visual_description", &path)?;
        let spatial_layout = required_string(scene, "spatial_layout", &path)?;
        let atmosphere = required_string(scene, "atmosphere", &path)?;
        let image_prompt_seed = scene_image_prompt_seed(
            &name,
            &base_description,
            &visual_description,
            &spatial_layout,
            &atmosphere,
        );
        drafts.push(SceneProfileDraft {
            scene_ref,
            scene_profile_id,
            name,
            aliases: string_array(scene, "aliases", &path, false)?,
            scene_type: scene_type_string(scene, "scene_type", &path)?,
            importance: importance_string(scene, "importance", &path)?,
            parent_scene_id,
            base_description,
            visual_description,
            spatial_layout,
            atmosphere,
            entry_points: required_array_value(scene, "entry_points", &path)?,
            rules: required_array_value(scene, "rules", &path)?,
            first_seen_chapter,
            last_seen_chapter,
            evidence,
            image_prompt_seed,
        });
    }
    Ok(ParsedSceneProfiles {
        drafts,
        evidence_refs,
    })
}

fn scene_type_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_scene_type_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            SCENE_TYPES.join("/")
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

fn scene_image_prompt_seed(
    name: &str,
    base_description: &str,
    visual_description: &str,
    spatial_layout: &str,
    atmosphere: &str,
) -> String {
    format!(
        "{name}，{base_description}，{visual_description}，空间布局：{spatial_layout}，氛围光线：{atmosphere}，可复用场景素材设定图"
    )
}

fn parse_scene_occurrences(
    root: &Value,
    context: &SceneProfileSyncRequestContext,
) -> Result<Vec<SceneOccurrenceDraft>, String> {
    let occurrences = required_array(root, "occurrences", "root")?;
    let mut refs = BTreeSet::new();
    let mut drafts = Vec::with_capacity(occurrences.len());
    for (index, occurrence) in occurrences.iter().enumerate() {
        let path = format!("occurrences[{index}]");
        ensure_object(occurrence, &path)?;
        reject_deprecated_evidence_fields(occurrence, &path, "source_anchor_ids")?;
        reject_unknown_fields(
            occurrence,
            &path,
            &[
                "occurrence_ref",
                "scene_ref",
                "timeline_position",
                "summary",
                "mood",
                "weather_or_lighting",
                "source_anchor_ids",
                "characters",
                "objects",
                "key_events",
                "entry_state",
                "exit_state",
            ],
        )?;
        let occurrence_ref = required_string(occurrence, "occurrence_ref", &path)?;
        if !refs.insert(occurrence_ref.clone()) {
            return Err(format!(
                "{path}.occurrence_ref 与前序出场重复: {occurrence_ref}"
            ));
        }
        let source_anchor_ids = positive_i64_array(occurrence, "source_anchor_ids", &path, true)?;
        let evidence = resolve_scene_text_anchor_texts(
            &format!("{path}.source_anchor_ids"),
            &source_anchor_ids,
            context,
        )?;
        drafts.push(SceneOccurrenceDraft {
            occurrence_ref,
            scene_ref: required_string(occurrence, "scene_ref", &path)?,
            occurrence_order: i32::try_from(index + 1)
                .map_err(|_| format!("{path}.occurrence_order 超出 i32 范围"))?,
            timeline_position: optional_string(occurrence, "timeline_position", &path)?,
            summary: required_string(occurrence, "summary", &path)?,
            mood: optional_string(occurrence, "mood", &path)?,
            weather_or_lighting: optional_string(occurrence, "weather_or_lighting", &path)?,
            source_anchor_ids,
            evidence,
            characters: validate_character_refs(
                occurrence,
                "characters",
                &path,
                &context.character_ids,
            )?,
            objects: validate_object_refs(occurrence, "objects", &path, &context.object_ids)?,
            key_events: string_array_value(occurrence, "key_events", &path, false)?,
            entry_state: required_object_value(occurrence, "entry_state", &path)?,
            exit_state: required_object_value(occurrence, "exit_state", &path)?,
        });
    }
    Ok(drafts)
}

fn parse_scene_transitions(
    root: &Value,
    context: &SceneProfileSyncRequestContext,
) -> Result<Vec<SceneTransitionDraft>, String> {
    let transitions = required_array(root, "transitions", "root")?;
    let mut drafts = Vec::with_capacity(transitions.len());
    for (index, transition) in transitions.iter().enumerate() {
        let path = format!("transitions[{index}]");
        ensure_object(transition, &path)?;
        reject_deprecated_evidence_fields(transition, &path, "source_anchor_ids")?;
        reject_unknown_fields(
            transition,
            &path,
            &[
                "from_scene_ref",
                "to_scene_ref",
                "occurrence_ref",
                "movement_type",
                "route_description",
                "time_cost",
                "distance_hint",
                "participants",
                "object_refs",
                "source_anchor_ids",
            ],
        )?;
        let source_anchor_ids = positive_i64_array(transition, "source_anchor_ids", &path, true)?;
        let evidence = resolve_scene_text_anchor_texts(
            &format!("{path}.source_anchor_ids"),
            &source_anchor_ids,
            context,
        )?;
        drafts.push(SceneTransitionDraft {
            from_scene_ref: optional_string(transition, "from_scene_ref", &path)?,
            to_scene_ref: required_string(transition, "to_scene_ref", &path)?,
            occurrence_ref: optional_string(transition, "occurrence_ref", &path)?,
            transition_order: i32::try_from(index + 1)
                .map_err(|_| format!("{path}.transition_order 超出 i32 范围"))?,
            movement_type: movement_type_string(transition, "movement_type", &path)?,
            route_description: required_string(transition, "route_description", &path)?,
            time_cost: optional_string(transition, "time_cost", &path)?,
            distance_hint: optional_string(transition, "distance_hint", &path)?,
            participants: id_array_value(
                transition,
                "participants",
                &path,
                &context.character_ids,
                "character_profile_id",
            )?,
            object_refs: id_array_value(
                transition,
                "object_refs",
                &path,
                &context.object_ids,
                "object_profile_id",
            )?,
            source_anchor_ids,
            evidence,
        });
    }
    Ok(drafts)
}

fn movement_type_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_movement_type_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            SCENE_TRANSITION_MOVEMENT_TYPES.join("/")
        )
    })
}

fn scene_entity_type_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_scene_entity_type_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            SCENE_ENTITY_TYPES.join("/")
        )
    })
}

fn scene_entity_state_key_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_scene_entity_state_key_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            SCENE_ENTITY_STATE_KEYS.join("/")
        )
    })
}

fn parse_scene_entity_states(
    root: &Value,
    context: &SceneProfileSyncRequestContext,
) -> Result<Vec<SceneEntityStateDraft>, String> {
    let entity_states = required_array(root, "entity_states", "root")?;
    let mut drafts = Vec::with_capacity(entity_states.len());
    for (index, state) in entity_states.iter().enumerate() {
        let path = format!("entity_states[{index}]");
        ensure_object(state, &path)?;
        reject_deprecated_evidence_fields(state, &path, "source_anchor_ids")?;
        reject_unknown_fields(
            state,
            &path,
            &[
                "scene_ref",
                "occurrence_ref",
                "entity_type",
                "entity_id",
                "state_key",
                "state_before",
                "state_after",
                "position_description",
                "action_description",
                "source_anchor_ids",
            ],
        )?;
        let entity_type = scene_entity_type_string(state, "entity_type", &path)?;
        let entity_id = optional_positive_i64(state, "entity_id", &path)?;
        match entity_type.as_str() {
            "character" => {
                let id = entity_id.ok_or_else(|| {
                    format!("{path}.entity_id 不能为空，character 必须引用已有角色 ID")
                })?;
                validate_id_in_set(id, &context.character_ids, &format!("{path}.entity_id"))?;
            }
            "object" => {
                let id = entity_id.ok_or_else(|| {
                    format!("{path}.entity_id 不能为空，object 必须引用已有物体档案 ID")
                })?;
                validate_id_in_set(id, &context.object_ids, &format!("{path}.entity_id"))?;
            }
            "event" => {
                if entity_id.is_some() {
                    return Err(format!("{path}.entity_id 必须为 null，event 没有档案 ID"));
                }
            }
            _ => unreachable!("validated enum"),
        }
        let action_description = required_string(state, "action_description", &path)?;
        let entity_name_snapshot = derive_entity_name_snapshot(
            &path,
            &entity_type,
            entity_id,
            &action_description,
            context,
        )?;
        let source_anchor_ids = positive_i64_array(state, "source_anchor_ids", &path, true)?;
        let evidence = resolve_scene_text_anchor_texts(
            &format!("{path}.source_anchor_ids"),
            &source_anchor_ids,
            context,
        )?;
        drafts.push(SceneEntityStateDraft {
            scene_ref: required_string(state, "scene_ref", &path)?,
            occurrence_ref: required_string(state, "occurrence_ref", &path)?,
            entity_type,
            entity_id,
            entity_name_snapshot,
            state_key: scene_entity_state_key_string(state, "state_key", &path)?,
            state_before: required_object_value(state, "state_before", &path)?,
            state_after: required_object_value(state, "state_after", &path)?,
            position_description: optional_string(state, "position_description", &path)?,
            action_description,
            source_anchor_ids,
            evidence,
        });
    }
    Ok(drafts)
}

fn derive_entity_name_snapshot(
    path: &str,
    entity_type: &str,
    entity_id: Option<i64>,
    action_description: &str,
    context: &SceneProfileSyncRequestContext,
) -> Result<String, String> {
    match entity_type {
        "character" => {
            let id = entity_id.ok_or_else(|| {
                format!("{path}.entity_id 不能为空，character 必须引用已有角色 ID")
            })?;
            context
                .character_names
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("{path}.entity_id={id} 缺少角色名称快照来源"))
        }
        "object" => {
            let id = entity_id.ok_or_else(|| {
                format!("{path}.entity_id 不能为空，object 必须引用已有物体档案 ID")
            })?;
            context
                .object_names
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("{path}.entity_id={id} 缺少物体名称快照来源"))
        }
        "event" => Ok(action_description.to_string()),
        _ => unreachable!("validated enum"),
    }
}

fn reject_deprecated_evidence_fields(
    value: &Value,
    path: &str,
    expected_field: &str,
) -> Result<(), String> {
    reject_deprecated_reference_fields(
        value,
        path,
        &[
            "evidence",
            "source_text",
            "original_text",
            "source_span_ids",
            "evidence_span_ids",
        ],
        expected_field,
    )
}

fn validate_character_refs(
    value: &Value,
    key: &str,
    path: &str,
    allowed_ids: &BTreeSet<i64>,
) -> Result<Value, String> {
    let refs = required_array(value, key, path)?;
    for (index, item) in refs.iter().enumerate() {
        let item_path = format!("{path}.{key}[{index}]");
        ensure_object(item, &item_path)?;
        let id = item
            .get("profile_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| format!("{item_path}.profile_id 必须是已有角色 ID"))?;
        validate_id_in_set(id, allowed_ids, &format!("{item_path}.profile_id"))?;
        required_string(item, "role", &item_path)?;
    }
    Ok(Value::Array(refs.clone()))
}

fn validate_object_refs(
    value: &Value,
    key: &str,
    path: &str,
    allowed_ids: &BTreeSet<i64>,
) -> Result<Value, String> {
    let refs = required_array(value, key, path)?;
    for (index, item) in refs.iter().enumerate() {
        let item_path = format!("{path}.{key}[{index}]");
        ensure_object(item, &item_path)?;
        let id = item
            .get("object_profile_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| format!("{item_path}.object_profile_id 必须是已有物体档案 ID"))?;
        validate_id_in_set(id, allowed_ids, &format!("{item_path}.object_profile_id"))?;
        required_string(item, "role", &item_path)?;
    }
    Ok(Value::Array(refs.clone()))
}

fn id_array_value(
    value: &Value,
    key: &str,
    path: &str,
    allowed_ids: &BTreeSet<i64>,
    label: &str,
) -> Result<Value, String> {
    let values = required_array(value, key, path)?;
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, item) in values.iter().enumerate() {
        let id = item
            .as_i64()
            .ok_or_else(|| format!("{path}.{key}[{index}] 必须是 {label} 整数 ID"))?;
        validate_id_in_set(id, allowed_ids, &format!("{path}.{key}[{index}]"))?;
        if seen.insert(id) {
            ids.push(id);
        }
    }
    Ok(json!(ids))
}

fn validate_id_in_set(id: i64, allowed_ids: &BTreeSet<i64>, path: &str) -> Result<(), String> {
    if id <= 0 {
        return Err(format!("{path} 必须大于 0"));
    }
    if !allowed_ids.contains(&id) {
        return Err(format!("{path} 引用了不在本次输入候选中的 ID: {id}"));
    }
    Ok(())
}

fn validate_scene_sync_references(input: &SceneSyncInput) -> Result<(), String> {
    let scene_refs = input
        .profiles
        .iter()
        .map(|profile| profile.scene_ref.as_str())
        .collect::<BTreeSet<_>>();
    let occurrence_scene_refs = input
        .occurrences
        .iter()
        .map(|occurrence| {
            (
                occurrence.occurrence_ref.as_str(),
                occurrence.scene_ref.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for profile in &input.profiles {
        if !input
            .occurrences
            .iter()
            .any(|occurrence| occurrence.scene_ref == profile.scene_ref)
        {
            return Err(format!(
                "scenes.scene_ref={} 没有对应 occurrences；场景档案必须记录本章出场",
                profile.scene_ref
            ));
        }
    }

    for (index, occurrence) in input.occurrences.iter().enumerate() {
        if !scene_refs.contains(occurrence.scene_ref.as_str()) {
            return Err(format!(
                "occurrences[{index}].scene_ref 引用了不存在的 scenes[].scene_ref: {}",
                occurrence.scene_ref
            ));
        }
    }

    for (index, transition) in input.transitions.iter().enumerate() {
        if let Some(from_scene_ref) = transition.from_scene_ref.as_deref()
            && !scene_refs.contains(from_scene_ref)
        {
            return Err(format!(
                "transitions[{index}].from_scene_ref 引用了不存在的 scenes[].scene_ref: {from_scene_ref}"
            ));
        }
        if !scene_refs.contains(transition.to_scene_ref.as_str()) {
            return Err(format!(
                "transitions[{index}].to_scene_ref 引用了不存在的 scenes[].scene_ref: {}",
                transition.to_scene_ref
            ));
        }
        if let Some(occurrence_ref) = transition.occurrence_ref.as_deref()
            && !occurrence_scene_refs.contains_key(occurrence_ref)
        {
            return Err(format!(
                "transitions[{index}].occurrence_ref 引用了不存在的 occurrences[].occurrence_ref: {occurrence_ref}"
            ));
        }
    }

    for (index, entity_state) in input.entity_states.iter().enumerate() {
        if !scene_refs.contains(entity_state.scene_ref.as_str()) {
            return Err(format!(
                "entity_states[{index}].scene_ref 引用了不存在的 scenes[].scene_ref: {}",
                entity_state.scene_ref
            ));
        }
        let Some(occurrence_scene_ref) =
            occurrence_scene_refs.get(entity_state.occurrence_ref.as_str())
        else {
            return Err(format!(
                "entity_states[{index}].occurrence_ref 引用了不存在的 occurrences[].occurrence_ref: {}",
                entity_state.occurrence_ref
            ));
        };
        if *occurrence_scene_ref != entity_state.scene_ref {
            return Err(format!(
                "entity_states[{index}].occurrence_ref 必须指向 scene_ref 对应的出场"
            ));
        }
    }

    Ok(())
}

fn resolve_scene_text_anchor_texts(
    path: &str,
    anchor_ids: &[i64],
    context: &SceneProfileSyncRequestContext,
) -> Result<Vec<String>, String> {
    anchor_quotes_in_exact_chapter(
        path,
        anchor_ids,
        &context.text_anchors,
        context.chapter.id,
        "当前章节",
    )
}

async fn bind_scene_profile_evidence(
    state: &AppState,
    project_id: i64,
    refs: &[SceneProfileEvidenceRefs],
    result: &a4n_services::scene::SceneSyncResult,
) -> Result<(), String> {
    for item in refs {
        let profile_id = result
            .profile_ref_ids
            .get(&item.scene_ref)
            .copied()
            .ok_or_else(|| format!("场景档案写入结果缺少 scene_ref: {}", item.scene_ref))?;
        let evidence_refs = item
            .evidence_anchor_ids
            .iter()
            .copied()
            .map(|text_anchor_id| ProfileEvidenceInput {
                text_anchor_id,
                evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
            })
            .collect::<Vec<_>>();
        state
            .text_anchor
            .merge_profile_evidence(
                project_id,
                None,
                PROFILE_KIND_SCENE,
                profile_id,
                evidence_refs,
            )
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn report_scene_profile_sync_progress(
    progress: Option<&SceneProfileSyncProgress<'_>>,
    current: u32,
    message: impl AsRef<str>,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.progress(current, SCENE_PROFILE_SYNC_PROGRESS_TOTAL, message.as_ref());
        progress.check_cancelled(cancelled_message)?;
    }
    Ok(())
}

fn scene_profile_result(profile: &scene_profile::Model) -> Value {
    json!({
        "id": profile.id,
        "project_id": profile.project_id,
        "name": profile.name,
        "normalized_name": profile.normalized_name,
        "aliases_json": profile.aliases_json,
        "scene_type": profile.scene_type,
        "importance": profile.importance,
        "parent_scene_id": profile.parent_scene_id,
        "base_description": profile.base_description,
        "visual_description": profile.visual_description,
        "spatial_layout": profile.spatial_layout,
        "atmosphere": profile.atmosphere,
        "entry_points_json": profile.entry_points_json,
        "rules_json": profile.rules_json,
        "current_state_json": profile.current_state_json,
        "first_seen_chapter": profile.first_seen_chapter,
        "last_seen_chapter": profile.last_seen_chapter,
        "evidence_json": profile.evidence_json,
        "image_prompt_seed": profile.image_prompt_seed,
        "created_at": profile.created_at.to_string(),
        "updated_at": profile.updated_at.to_string(),
    })
}

#[cfg(test)]
mod tests;
