use crate::{
    actions::{
        access,
        character_implicit_review::IMPLICIT_UPDATE_PROMPT_NAME,
        json_contract::reject_unknown_fields,
        llm_request::{
            ContextAwareJsonLlmRequestInput, PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT,
            context_aware_system_user_json_llm_request,
        },
    },
    state::AppState,
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::models::character::{attribute_event, protagonist_profile};
use a4n_engine::llm::provider::ChatRequest;
use a4n_engine::prompt::request::build_json_task_user_message;
use a4n_services::character::apply_profile_current_state;
use a4n_services::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunInput,
    context_aware_chat_request_trace_json, record_completed_generation_run,
};
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::{Value, json};
use std::collections::HashMap;

use super::json_utils::{required_array, required_i32_at, required_string, required_string_array};
use super::snapshot_storage::{StoreChapterSnapshotsInput, store_chapter_snapshots};
use super::state_projection::pending_attribute_events;
use super::sync_context::{CharacterSyncAnchorMode, build_character_sync_context_by_name};
use super::sync_implicit::{classify_behaviors, decide_implicit_reviews};
use super::sync_llm::ensure_character_sync_response_not_truncated;
use super::sync_response::{
    CHARACTER_SYNC_ATTRIBUTE_CHANGES_PER_CHAPTER_MAX, CHARACTER_SYNC_BEHAVIORS_PER_CHAPTER_MAX,
    CHARACTER_SYNC_DELETION_CANDIDATES_PER_CHAPTER_MAX, ChapterStateDraft, chapter_state_drafts,
    validate_attribute_change, validate_behavior, validate_chapter_state,
    validate_deletion_candidate, validate_unique_deletion_candidates,
};
use super::sync_scope::{profile_sync_skip_reason, sync_skip_summary};
use super::sync_trace::record_character_sync_llm_trace;
use super::sync_types::{
    CHARACTER_PROFILE_SYNC_STEPS, CharacterProgressScope, CharacterSyncContext,
    CharacterSyncExtraction,
};
use super::sync_validate::{
    sort_sync_items_by_chapter, validate_materialized_character_sync_projection,
    validate_sync_evidence_against_chapters, validate_sync_item_chapters,
    validate_sync_item_count_by_chapter,
};
use super::sync_write::{ApplySyncWritesInput, apply_sync_writes_by_chapter};
use super::{CharacterAiPreviewStep, CharacterAiProgress};

pub(super) struct CharacterSyncGenerationTrace {
    request_json: Value,
    rendered_messages_json: Value,
    prompt_template_names_json: Value,
    response_chars: usize,
}

impl CharacterSyncGenerationTrace {
    fn from_stage_exchanges(exchanges: &[CharacterSyncStageExchange]) -> Result<Self, String> {
        let stage_requests = exchanges
            .iter()
            .map(|exchange| {
                context_aware_chat_request_trace_json(&exchange.request).map(|request| {
                    json!({
                        "prompt_name": exchange.stage.prompt_name(),
                        "request": request,
                    })
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("角色同步分阶段 LLM 请求序列化失败: {err}"))?;
        let stage_messages = exchanges
            .iter()
            .map(|exchange| {
                serde_json::to_value(&exchange.request.messages).map(|messages| {
                    json!({
                        "prompt_name": exchange.stage.prompt_name(),
                        "messages": messages,
                    })
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("角色同步分阶段 LLM 消息序列化失败: {err}"))?;
        Ok(Self {
            request_json: json!({ "stages": stage_requests }),
            rendered_messages_json: json!({ "stages": stage_messages }),
            prompt_template_names_json: json!(
                exchanges
                    .iter()
                    .map(|exchange| exchange.stage.prompt_name())
                    .collect::<Vec<_>>()
            ),
            response_chars: exchanges
                .iter()
                .map(|exchange| exchange.response_content.chars().count())
                .sum(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CharacterSyncStage {
    EvidenceEvents,
    AttributeChanges,
    ChapterStates,
    BehaviorRecords,
    DeletionCandidates,
}

impl CharacterSyncStage {
    const ALL: [Self; 5] = [
        Self::EvidenceEvents,
        Self::AttributeChanges,
        Self::ChapterStates,
        Self::BehaviorRecords,
        Self::DeletionCandidates,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::EvidenceEvents => "character_sync_evidence_events",
            Self::AttributeChanges => "character_sync_attribute_changes",
            Self::ChapterStates => "character_sync_chapter_states",
            Self::BehaviorRecords => "character_sync_behavior_records",
            Self::DeletionCandidates => "character_sync_deletion_candidates",
        }
    }

    fn prompt_name(self) -> &'static str {
        self.key()
    }

    fn title(self) -> &'static str {
        match self {
            Self::EvidenceEvents => "角色同步 Step 1：证据事件抽取",
            Self::AttributeChanges => "角色同步 Step 2：属性变化判断",
            Self::ChapterStates => "角色同步 Step 3：章节状态生成",
            Self::BehaviorRecords => "角色同步 Step 4：关键行为记录",
            Self::DeletionCandidates => "角色同步 Step 5：删除候选判断",
        }
    }

    fn progress_message(self, name: &str) -> String {
        match self {
            Self::EvidenceEvents => format!("{name}：正在抽取角色相关证据事件"),
            Self::AttributeChanges => format!("{name}：正在判断角色属性变化"),
            Self::ChapterStates => format!("{name}：正在生成章节状态"),
            Self::BehaviorRecords => format!("{name}：正在记录关键行为"),
            Self::DeletionCandidates => format!("{name}：正在判断删除候选"),
        }
    }

    fn root_field(self) -> &'static str {
        match self {
            Self::EvidenceEvents => "events",
            Self::AttributeChanges => "attribute_changes",
            Self::ChapterStates => "chapter_states",
            Self::BehaviorRecords => "behaviors",
            Self::DeletionCandidates => "deletion_candidates",
        }
    }
}

struct CharacterSyncStageExchange {
    stage: CharacterSyncStage,
    request: ChatRequest,
    response_content: String,
}

struct CharacterSyncStageRun {
    extraction: CharacterSyncExtraction,
    generation_trace: CharacterSyncGenerationTrace,
    llm_trace_paths: Vec<String>,
}

#[derive(Default)]
struct CharacterSyncStageOutputs {
    evidence_events: Vec<Value>,
    changes: Vec<Value>,
    behaviors: Vec<Value>,
    deletion_candidates: Vec<Value>,
    chapter_states: Vec<ChapterStateDraft>,
}

fn validate_extraction_evidence(
    mut extraction: CharacterSyncExtraction,
    context: &CharacterSyncContext,
) -> Result<CharacterSyncExtraction, String> {
    validate_sync_evidence_against_chapters(
        &mut extraction.changes,
        &mut extraction.behaviors,
        &mut extraction.deletion_candidates,
        &context.chapter_data,
    )?;
    validate_materialized_character_sync_projection(&extraction, &context.baseline_state)?;
    Ok(extraction)
}

fn character_sync_stage_preview_steps_for_context(
    state: &AppState,
    project_id: i64,
    context: &CharacterSyncContext,
    name: &str,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    let mut steps = Vec::new();
    let mut previous_steps = json!({});
    for (index, stage) in CharacterSyncStage::ALL.iter().copied().enumerate() {
        if index == 0 {
            steps.push(CharacterAiPreviewStep::llm(
                format!("sync_{}_{}", stage.key(), context.profile.id),
                format!("{name}：{}", stage.title()),
                stage.prompt_name(),
                "ready",
                Some(format!(
                    "同步第 {}-{} 章；后续阶段依赖该阶段输出",
                    context.start_chapter, context.target_chapter
                )),
                build_character_sync_stage_request(
                    state,
                    project_id,
                    context,
                    stage,
                    &previous_steps,
                )?,
            ));
        } else {
            steps.push(CharacterAiPreviewStep::status(
                format!("sync_{}_{}", stage.key(), context.profile.id),
                format!("{name}：{}", stage.title()),
                stage.prompt_name(),
                "blocked",
                Some("该阶段依赖前序角色同步阶段的结构化输出".to_string()),
            ));
        }
        previous_steps[stage.key()] = json!({ stage.root_field(): [] });
    }
    steps.push(CharacterAiPreviewStep::status(
        format!("sync_implicit_classification_{}", context.profile.id),
        format!("{name}：行为隐性属性分类请求"),
        "implicit_classification",
        "blocked",
        Some("该请求依赖角色同步 behavior_records 阶段输出".to_string()),
    ));
    steps.push(CharacterAiPreviewStep::status(
        format!("sync_implicit_review_{}", context.profile.id),
        format!("{name}：隐性属性复审请求"),
        IMPLICIT_UPDATE_PROMPT_NAME,
        "blocked",
        Some("该请求依赖角色同步阶段输出和行为分类结果".to_string()),
    ));
    Ok(steps)
}

async fn run_staged_character_sync(
    state: &AppState,
    project_id: i64,
    workflow_run_id: Option<i64>,
    context: &CharacterSyncContext,
    progress: CharacterProgressScope<'_>,
) -> Result<CharacterSyncStageRun, String> {
    let name = context.profile.name.as_str();
    let mut outputs = CharacterSyncStageOutputs::default();
    let mut previous_steps = json!({});
    let mut exchanges = Vec::new();
    let mut llm_trace_paths = Vec::new();

    for (index, stage) in CharacterSyncStage::ALL.iter().copied().enumerate() {
        let current = 1 + index as u32;
        progress.report(
            current,
            CHARACTER_PROFILE_SYNC_STEPS,
            &stage.progress_message(name),
        )?;
        let mut request =
            build_character_sync_stage_request(state, project_id, context, stage, &previous_steps)?;
        let call_key = format!(
            "character_sync:{}:{}:{}",
            context.profile.id,
            context.target_chapter,
            stage.key()
        );
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)
            .map_err(|err| err.to_string())?;
        let request_json = context_aware_chat_request_trace_json(&request)
            .map_err(|err| format!("角色同步阶段 LLM 请求序列化失败: {err}"))?;
        let input_fingerprint_json = json!({
            "request": request_json,
            "stage": stage.key(),
            "profile_id": context.profile.id,
            "start_chapter": context.start_chapter,
            "target_chapter": context.target_chapter,
            "previous_steps": &previous_steps,
        });
        let input_fingerprint_sha256 =
            llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                .map_err(|err| err.to_string())?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        state
            .workflow_run
            .mark_current_step(workflow_run_id, &call_key)
            .await
            .map_err(|err| err.to_string())?;

        let (response_content, response_usage_json, response_finish_reason, restored_parsed_output) =
            if let Some(checkpoint) = load_completed_llm_call_checkpoint(
                state.db.clone(),
                workflow_run_id,
                &call_key,
                &input_fingerprint_json,
            )
            .await
            .map_err(|err| err.to_string())?
            {
                let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(|err| err.to_string())?;
                let parsed_output = checkpoint_json
                    .get("parsed_output")
                    .cloned()
                    .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
                let response_content = required_character_sync_checkpoint_string(
                    &checkpoint_json,
                    &call_key,
                    "response_content",
                )?;
                let finish_reason =
                    optional_character_sync_checkpoint_string(&checkpoint_json, "finish_reason")?;
                ensure_character_sync_response_not_truncated(
                    finish_reason.as_deref(),
                    &format!("{name} {}", stage.title()),
                )?;
                let usage = checkpoint_json
                    .get("usage")
                    .filter(|value| !value.is_null())
                    .cloned();
                (response_content, usage, finish_reason, Some(parsed_output))
            } else {
                let response = state
                    .llm
                    .chat(request.clone())
                    .await
                    .map_err(|e| e.to_string())?;
                ensure_character_sync_response_not_truncated(
                    response.finish_reason.as_deref(),
                    &format!("{name} {}", stage.title()),
                )?;
                let response_usage_json = response
                    .usage
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|err| format!("角色同步阶段 LLM 用量序列化失败: {err}"))?;
                (
                    response.message.content,
                    response_usage_json,
                    response.finish_reason,
                    None,
                )
            };
        match parse_character_sync_stage_output(stage, &response_content, context, &mut outputs) {
            Ok(stage_value) => {
                if let Some(parsed_output) = restored_parsed_output {
                    if parsed_output != stage_value {
                        return Err(format!(
                            "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
                        ));
                    }
                } else {
                    save_completed_llm_call_checkpoint(
                        state.db.clone(),
                        CompletedLlmCallCheckpointInput {
                            workflow_run_id,
                            call_key: call_key.clone(),
                            call_order: 1 + index as i32,
                            input_fingerprint_json: input_fingerprint_json.clone(),
                            checkpoint_json: json!({
                                "output": stage_value.clone(),
                                "parsed_output": stage_value.clone(),
                                "response_content": response_content.clone(),
                                "usage": response_usage_json,
                                "finish_reason": response_finish_reason,
                            }),
                            artifacts_json: None,
                        },
                    )
                    .await
                    .map_err(|err| err.to_string())?;
                }
                if let Some(path) = record_character_sync_llm_trace(
                    state,
                    name,
                    &format!("{}_success", stage.key()),
                    &request.messages,
                    &response_content,
                    Some(stage_temperature(stage)),
                    None,
                ) {
                    llm_trace_paths.push(path);
                }
                previous_steps[stage.key()] = stage_value;
                exchanges.push(CharacterSyncStageExchange {
                    stage,
                    request,
                    response_content,
                });
            }
            Err(error) => {
                if let Some(path) = record_character_sync_llm_trace(
                    state,
                    name,
                    &format!("{}_invalid", stage.key()),
                    &request.messages,
                    &response_content,
                    Some(stage_temperature(stage)),
                    Some(&error),
                ) {
                    llm_trace_paths.push(path);
                }
                let trace_summary = if llm_trace_paths.is_empty() {
                    String::new()
                } else {
                    format!("；LLM 调试日志：{}", llm_trace_paths.join("，"))
                };
                return Err(format!(
                    "{}输出不符合角色同步阶段契约：{error}{trace_summary}",
                    stage.title()
                ));
            }
        }
    }

    progress.report(
        6,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：正在组合并校验角色同步阶段结果"),
    )?;
    let extraction = validate_extraction_evidence(
        CharacterSyncExtraction {
            changes: outputs.changes,
            behaviors: outputs.behaviors,
            deletion_candidates: outputs.deletion_candidates,
            chapter_states: outputs.chapter_states,
        },
        context,
    )?;
    let generation_trace = CharacterSyncGenerationTrace::from_stage_exchanges(&exchanges)?;
    Ok(CharacterSyncStageRun {
        extraction,
        generation_trace,
        llm_trace_paths,
    })
}

fn required_character_sync_checkpoint_string(
    checkpoint_json: &Value,
    call_key: &str,
    field: &str,
) -> Result<String, String> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.{field} 必须是字符串"))
}

fn optional_character_sync_checkpoint_string(
    checkpoint_json: &Value,
    field: &str,
) -> Result<Option<String>, String> {
    match checkpoint_json.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_str()
            .map(ToString::to_string)
            .map(Some)
            .ok_or_else(|| format!("{field} 必须是字符串或 null")),
    }
}

fn stage_temperature(stage: CharacterSyncStage) -> f32 {
    match stage {
        CharacterSyncStage::EvidenceEvents => 0.2,
        CharacterSyncStage::AttributeChanges => 0.25,
        CharacterSyncStage::ChapterStates => 0.25,
        CharacterSyncStage::BehaviorRecords => 0.25,
        CharacterSyncStage::DeletionCandidates => 0.15,
    }
}

fn build_character_sync_stage_request(
    state: &AppState,
    project_id: i64,
    context: &CharacterSyncContext,
    stage: CharacterSyncStage,
    previous_steps: &Value,
) -> Result<ChatRequest, String> {
    let system_prompt = state
        .engine
        .render(stage.prompt_name(), &HashMap::new(), Some(project_id))
        .map_err(|err| err.to_string())?;
    let user_prompt = build_character_sync_stage_user_prompt(stage, previous_steps)?;
    context_aware_system_user_json_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id,
            workflow_run_id: None,
            workflow: "character_sync".to_string(),
            step_key: stage.key().to_string(),
            scope_kind: "profile_sync".to_string(),
            pack_kind: PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "profile_id": context.profile.id,
                "profile_name": context.profile.name,
                "start_chapter": context.start_chapter,
                "target_chapter": context.target_chapter,
                "input_chapter_numbers": context.selected_chapter_numbers,
            })),
            config_json: json!({
                "ledger_version": "character_sync_context_v1",
                "prompt_name": stage.prompt_name(),
                "stage": stage.key(),
            }),
            stable_context_json: character_sync_stage_stable_context(context),
            dynamic_user_prompt: user_prompt,
            temperature: Some(stage_temperature(stage)),
            max_tokens_profile: llm_token_profiles::CHARACTER_SYNC.to_string(),
            call_key: Some(format!(
                "character_sync:{}:{}:{}",
                context.profile.id,
                context.target_chapter,
                stage.key()
            )),
        },
    )
}

fn build_character_sync_stage_user_prompt(
    stage: CharacterSyncStage,
    previous_steps: &Value,
) -> Result<String, String> {
    let payload = json!({
        "context_refs": {
            "source": format!("稳定上下文包 {PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT}")
        },
        "task": {
            "name": stage.key(),
            "instruction": stage.title(),
        },
        "output_contract": character_sync_stage_contract(stage),
        "previous_steps": previous_steps,
    });
    build_json_task_user_message(
        stage.title(),
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|err| format!("角色同步阶段请求构造失败: {err}"))
}

fn character_sync_stage_stable_context(context: &CharacterSyncContext) -> Value {
    json!({
        "character": {
            "profile_id": context.profile.id,
            "profile_name": &context.profile.name,
            "last_synced_chapter": context.profile.last_synced_chapter,
        },
        "sync_range": {
            "start_chapter": context.start_chapter,
            "target_chapter": context.target_chapter,
            "input_chapter_numbers": &context.selected_chapter_numbers,
        },
        "baseline_state": &context.baseline_state,
        "base_attributes": &context.profile.base_attributes,
        "chapters": &context.chapter_data,
    })
}

pub(super) fn character_sync_stage_contract(stage: CharacterSyncStage) -> Value {
    match stage {
        CharacterSyncStage::EvidenceEvents => json!({
            "format": "json_object",
            "required_root_field": "events",
            "item_required_fields": ["chapter_number", "event_type", "summary", "evidence_anchor_id", "related_characters"],
            "rules": [
                "只抽取当前角色在本次章节中的事实事件、状态变化线索、关键行为线索和可能失效线索",
                "chapter_number 必须来自 input_chapter_numbers",
                "evidence_anchor_id 必须来自对应章节 chapters[].text_anchors[].id",
                "related_characters 没有时输出 []"
            ]
        }),
        CharacterSyncStage::AttributeChanges => json!({
            "format": "json_object",
            "required_root_field": "attribute_changes",
            "item_required_fields": ["chapter_number", "category", "key", "operation", "new_value", "change_description", "event_cause", "evidence_anchor_id"],
            "field_value_rules": {
                "category": {
                    "allowed_values": ["explicit", "implicit", "social"],
                    "forbidden_values": ["explicit_attributes", "implicit_attributes", "social_attributes"],
                    "mapping": {
                        "explicit": "baseline_state.explicit_attributes",
                        "implicit": "baseline_state.implicit_attributes",
                        "social": "baseline_state.social_attributes"
                    }
                },
                "operation": {
                    "allowed_values": ["add", "modify"],
                    "forbidden_values": ["delete"]
                }
            },
            "rules": [
                "只基于 previous_steps.character_sync_evidence_events 判断属性新增或修改",
                "不要输出 old_value、updated_attributes、current_state、delete 操作或最终状态快照",
                "category 必须只输出 explicit、implicit、social；不要输出 explicit_attributes、implicit_attributes、social_attributes",
                "key 必须是 category 对应属性对象下的顶层键，不能使用点路径",
                "没有属性变化时输出 []"
            ]
        }),
        CharacterSyncStage::ChapterStates => json!({
            "format": "json_object",
            "required_root_field": "chapter_states",
            "item_required_fields": ["chapter_number", "state_summary", "current_location", "current_goal", "emotional_state"],
            "rules": [
                "chapter_states 必须覆盖 input_chapter_numbers 中每个章节且不能重复",
                "只输出章节结束时该角色的稳定状态，不要输出属性变化或行为记录",
                "current_location、current_goal、emotional_state 不明确时输出 null"
            ]
        }),
        CharacterSyncStage::BehaviorRecords => json!({
            "format": "json_object",
            "required_root_field": "behaviors",
            "item_required_fields": ["chapter_number", "behavior_type", "behavior_description", "original_text_anchor_id", "behavior_tags", "related_characters", "impact_analysis"],
            "rules": [
                "只记录推动剧情、暴露性格、改变关系或影响目标的关键行为",
                "不要输出 classification_results 或 original_text",
                "impact_analysis 没有实质影响时输出空字符串"
            ]
        }),
        CharacterSyncStage::DeletionCandidates => json!({
            "format": "json_object",
            "required_root_field": "deletion_candidates",
            "item_required_fields": ["chapter_number", "category", "key", "reason", "evidence_anchor_id"],
            "field_value_rules": {
                "category": {
                    "allowed_values": ["explicit", "implicit", "social"],
                    "forbidden_values": ["explicit_attributes", "implicit_attributes", "social_attributes"],
                    "mapping": {
                        "explicit": "baseline_state.explicit_attributes",
                        "implicit": "baseline_state.implicit_attributes",
                        "social": "baseline_state.social_attributes"
                    }
                }
            },
            "rules": [
                "只记录已有属性被明确否定、失效或需要删除保护的候选",
                "不要输出 attribute_changes，也不要直接执行删除",
                "category 必须只输出 explicit、implicit、social；不要输出 explicit_attributes、implicit_attributes、social_attributes",
                "没有删除候选时输出 []"
            ]
        }),
    }
}

fn parse_character_sync_stage_output(
    stage: CharacterSyncStage,
    content: &str,
    context: &CharacterSyncContext,
    outputs: &mut CharacterSyncStageOutputs,
) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim())
        .map_err(|err| format!("{}必须是 JSON 对象: {err}", stage.title()))?;
    if !value.is_object() {
        return Err(format!("{}必须是 JSON 对象", stage.title()));
    }
    reject_unknown_fields(&value, "root", &[stage.root_field()])?;
    match stage {
        CharacterSyncStage::EvidenceEvents => {
            outputs.evidence_events = parse_sync_evidence_events(&value, context)?;
            Ok(json!({ "events": outputs.evidence_events }))
        }
        CharacterSyncStage::AttributeChanges => {
            outputs.changes = parse_sync_attribute_changes(&value, context)?;
            Ok(json!({ "attribute_changes": outputs.changes }))
        }
        CharacterSyncStage::ChapterStates => {
            outputs.chapter_states = parse_sync_chapter_states(&value, context)?;
            Ok(json!({ "chapter_states": chapter_state_drafts_json(&outputs.chapter_states) }))
        }
        CharacterSyncStage::BehaviorRecords => {
            outputs.behaviors = parse_sync_behaviors(&value, context)?;
            Ok(json!({ "behaviors": outputs.behaviors }))
        }
        CharacterSyncStage::DeletionCandidates => {
            outputs.deletion_candidates = parse_sync_deletion_candidates(&value, context)?;
            Ok(json!({ "deletion_candidates": outputs.deletion_candidates }))
        }
    }
}

fn parse_sync_evidence_events(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<Value>, String> {
    let events = required_array(value, "events")?.clone();
    validate_sync_item_chapters("events", &events, &context.selected_chapter_numbers)?;
    for (index, event) in events.iter().enumerate() {
        if !event.is_object() {
            return Err(format!("events[{index}] 必须是 JSON 对象"));
        }
        required_i32_at(
            event,
            "chapter_number",
            &format!("events[{index}].chapter_number"),
        )?;
        required_string(event, "event_type")?;
        required_string(event, "summary")?;
        required_string_array(event, "related_characters")?;
        ensure_stage_anchor_exists(event, "events", index, "evidence_anchor_id", context)?;
    }
    Ok(events)
}

fn parse_sync_attribute_changes(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<Value>, String> {
    let mut changes = required_array(value, "attribute_changes")?.clone();
    for (index, change) in changes.iter().enumerate() {
        validate_attribute_change(index, change)?;
    }
    validate_sync_item_chapters(
        "attribute_changes",
        &changes,
        &context.selected_chapter_numbers,
    )?;
    validate_sync_item_count_by_chapter(
        "attribute_changes",
        &changes,
        CHARACTER_SYNC_ATTRIBUTE_CHANGES_PER_CHAPTER_MAX,
    )?;
    sort_sync_items_by_chapter(&mut changes)?;
    Ok(changes)
}

fn parse_sync_chapter_states(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<ChapterStateDraft>, String> {
    for (index, state) in required_array(value, "chapter_states")?.iter().enumerate() {
        validate_chapter_state(index, state)?;
    }
    chapter_state_drafts(value, &context.selected_chapter_numbers)
}

fn chapter_state_drafts_json(states: &[ChapterStateDraft]) -> Value {
    Value::Array(
        states
            .iter()
            .map(|state| {
                json!({
                    "chapter_number": state.chapter_number,
                    "state_summary": &state.state_summary,
                    "current_location": &state.current_location,
                    "current_goal": &state.current_goal,
                    "emotional_state": &state.emotional_state,
                })
            })
            .collect(),
    )
}

fn parse_sync_behaviors(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<Value>, String> {
    let mut behaviors = required_array(value, "behaviors")?.clone();
    for (index, behavior) in behaviors.iter().enumerate() {
        validate_behavior(index, behavior)?;
    }
    validate_sync_item_chapters("behaviors", &behaviors, &context.selected_chapter_numbers)?;
    validate_sync_item_count_by_chapter(
        "behaviors",
        &behaviors,
        CHARACTER_SYNC_BEHAVIORS_PER_CHAPTER_MAX,
    )?;
    sort_sync_items_by_chapter(&mut behaviors)?;
    Ok(behaviors)
}

fn parse_sync_deletion_candidates(
    value: &Value,
    context: &CharacterSyncContext,
) -> Result<Vec<Value>, String> {
    let mut candidates = required_array(value, "deletion_candidates")?.clone();
    for (index, candidate) in candidates.iter().enumerate() {
        validate_deletion_candidate(index, candidate)?;
    }
    validate_unique_deletion_candidates(&candidates)?;
    validate_sync_item_chapters(
        "deletion_candidates",
        &candidates,
        &context.selected_chapter_numbers,
    )?;
    validate_sync_item_count_by_chapter(
        "deletion_candidates",
        &candidates,
        CHARACTER_SYNC_DELETION_CANDIDATES_PER_CHAPTER_MAX,
    )?;
    sort_sync_items_by_chapter(&mut candidates)?;
    Ok(candidates)
}

fn ensure_stage_anchor_exists(
    item: &Value,
    label: &str,
    index: usize,
    anchor_field: &str,
    context: &CharacterSyncContext,
) -> Result<(), String> {
    let chapter_number = required_i32_at(
        item,
        "chapter_number",
        &format!("{label}[{index}].chapter_number"),
    )?;
    let anchor_id = item
        .get(anchor_field)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}[{index}].{anchor_field} 必须是 JSON 整数数字"))?;
    if anchor_id <= 0 {
        return Err(format!("{label}[{index}].{anchor_field} 必须大于 0"));
    }
    let chapter = context
        .chapter_data
        .iter()
        .find(|chapter| {
            chapter
                .get("chapter_number")
                .and_then(Value::as_i64)
                .is_some_and(|value| value == i64::from(chapter_number))
        })
        .ok_or_else(|| format!("{label}[{index}] 缺少可核对证据的第 {chapter_number} 章正文"))?;
    let exists = chapter
        .get("text_anchors")
        .and_then(Value::as_array)
        .is_some_and(|anchors| {
            anchors.iter().any(|anchor| {
                anchor
                    .get("id")
                    .and_then(Value::as_i64)
                    .is_some_and(|value| value == anchor_id)
            })
        });
    if !exists {
        return Err(format!(
            "{label}[{index}].{anchor_field} 不在第 {chapter_number} 章 text_anchors 中：{anchor_id}"
        ));
    }
    Ok(())
}

pub async fn sync_character_profile_from_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    chapter_number: i32,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    sync_character_profile_from_chapters_with_scope(
        state,
        project_id,
        name,
        chapter_number,
        CharacterProgressScope::root(progress, CHARACTER_PROFILE_SYNC_STEPS, "角色同步已停止"),
    )
    .await
}

pub(crate) async fn character_sync_preview_steps(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    chapter_number: i32,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let context = build_character_sync_context_by_name(
        state,
        project_id,
        name,
        chapter_number,
        CharacterSyncAnchorMode::RequireExistingCurrent,
    )
    .await?;
    character_sync_preview_steps_for_context(state, project_id, &context, name)
}

pub(super) fn character_sync_preview_steps_for_context(
    state: &AppState,
    project_id: i64,
    context: &CharacterSyncContext,
    name: &str,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    if profile_sync_skip_reason(&context.profile, context.target_chapter).is_some() {
        return Ok(vec![CharacterAiPreviewStep::status(
            format!("sync_{}", context.profile.id),
            format!("{name}：无需同步"),
            "character_sync_evidence_events",
            "reused",
            Some(format!(
                "已同步到第 {} 章或目标章节无需处理",
                context.target_chapter
            )),
        )]);
    }

    character_sync_stage_preview_steps_for_context(state, project_id, context, name)
}

pub(super) async fn sync_character_profile_from_chapters_with_scope(
    state: &AppState,
    project_id: i64,
    name: &str,
    chapter_number: i32,
    progress: CharacterProgressScope<'_>,
) -> Result<Value, String> {
    progress.report(
        0,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：正在读取角色档案和可同步章节"),
    )?;
    let context = build_character_sync_context_by_name(
        state,
        project_id,
        name,
        chapter_number,
        CharacterSyncAnchorMode::EnsureCurrent,
    )
    .await?;
    sync_character_profile_context_with_scope(state, project_id, context, progress).await
}

pub(super) async fn sync_character_profile_context_with_scope(
    state: &AppState,
    project_id: i64,
    context: CharacterSyncContext,
    progress: CharacterProgressScope<'_>,
) -> Result<Value, String> {
    let name = context.profile.name.clone();
    if profile_sync_skip_reason(&context.profile, context.target_chapter).is_some() {
        progress.report(
            CHARACTER_PROFILE_SYNC_STEPS,
            CHARACTER_PROFILE_SYNC_STEPS,
            &format!("{name}：无需同步到第 {} 章，跳过", context.target_chapter),
        )?;
        return Ok(sync_skip_summary(&context.profile, context.target_chapter));
    }

    let workflow_fingerprint = character_sync_workflow_fingerprint(project_id, &context);
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!(
                "character_profile_sync:{}:{}",
                context.profile.id, context.target_chapter
            ),
            workflow_kind: "character_profile_sync".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(context.profile.id),
            chapter_id: None,
            mode: Some(format!(
                "{}-{}",
                context.start_chapter, context.target_chapter
            )),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("character_sync.start".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let staged =
            run_staged_character_sync(state, project_id, Some(workflow_run_id), &context, progress)
                .await?;
        apply_character_sync_extraction(
            state,
            project_id,
            Some(workflow_run_id),
            context,
            staged.extraction,
            &staged.generation_trace,
            staged.llm_trace_paths,
            progress,
        )
        .await
    }
    .await;

    finalize_character_sync_workflow(state, workflow_run_id, &result).await?;
    result
}

fn character_sync_workflow_fingerprint(project_id: i64, context: &CharacterSyncContext) -> Value {
    json!({
        "project_id": project_id,
        "profile_id": context.profile.id,
        "profile_name": &context.profile.name,
        "last_synced_chapter": context.profile.last_synced_chapter,
        "start_chapter": context.start_chapter,
        "target_chapter": context.target_chapter,
        "clear_to_chapter": context.clear_to_chapter,
        "selected_chapter_numbers": &context.selected_chapter_numbers,
        "baseline_state": &context.baseline_state,
        "base_attributes": &context.profile.base_attributes,
        "chapter_data": &context.chapter_data,
    })
}

async fn finalize_character_sync_workflow(
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
        .map(|_| ())
        .map_err(|err| err.to_string())
}

pub(super) async fn apply_character_sync_extraction(
    state: &AppState,
    project_id: i64,
    workflow_run_id: Option<i64>,
    context: CharacterSyncContext,
    extraction: CharacterSyncExtraction,
    generation_trace: &CharacterSyncGenerationTrace,
    llm_trace_paths: Vec<String>,
    progress: CharacterProgressScope<'_>,
) -> Result<Value, String> {
    let CharacterSyncContext {
        profile,
        start_chapter,
        target_chapter,
        clear_to_chapter,
        selected_chapter_numbers,
        chapter_data,
        baseline_state,
    } = context;
    let name = profile.name.clone();
    let profile_last_synced_chapter = profile.last_synced_chapter;
    let selected_chapter_count = selected_chapter_numbers.len();
    let changes = extraction.changes;
    let mut behaviors = extraction.behaviors;
    let deletion_candidates = extraction.deletion_candidates;
    let chapter_states = extraction.chapter_states;
    let mut final_attributes = baseline_state.clone();
    progress.report(
        7,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：正在识别行为类型并检查隐性属性变化"),
    )?;
    let classification_suggestions = classify_behaviors(
        state,
        project_id,
        workflow_run_id,
        &profile.name,
        &changes,
        &mut behaviors,
        &mut final_attributes,
    )
    .await?;
    let implicit_review_decisions = decide_implicit_reviews(
        state,
        workflow_run_id,
        &profile,
        start_chapter,
        &final_attributes,
        &behaviors,
        &deletion_candidates,
    )
    .await?;

    let profile_id = profile.id;
    progress.report(
        8,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：正在写入角色事件、状态索引和快照"),
    )?;
    let txn = state.db.begin().await.map_err(|e| e.to_string())?;

    let deleted_timeline = state
        .character
        .delete_timeline_artifacts_from_chapter(
            &txn,
            profile_id,
            start_chapter,
            Some(clear_to_chapter),
        )
        .await
        .map_err(|e| e.to_string())?;
    let stale_state_index_ids = deleted_timeline.state_index_ids.clone();
    let stale_attribute_event_ids = deleted_timeline.attribute_event_ids.clone();
    let deletion_mark_rewind = state
        .character
        .rewind_deletion_marks_after(&txn, profile_id, start_chapter - 1)
        .await
        .map_err(|e| e.to_string())?;
    let mut baseline_profile: protagonist_profile::ActiveModel = profile.clone().into();
    apply_profile_current_state(&mut baseline_profile, baseline_state.clone());
    baseline_profile.updated_at = Set(chrono::Utc::now().naive_utc());
    baseline_profile
        .update(&txn)
        .await
        .map_err(|e| e.to_string())?;

    let pending_attribute_events =
        pending_attribute_events(&baseline_state, &changes, &classification_suggestions)?;
    let write_summary = apply_sync_writes_by_chapter(ApplySyncWritesInput {
        state,
        db: &txn,
        profile_id: profile.id,
        chapter_numbers: &selected_chapter_numbers,
        attribute_events: &pending_attribute_events,
        behaviors: &behaviors,
        deletion_candidates: &deletion_candidates,
        implicit_review_decisions: &implicit_review_decisions,
        target_chapter,
    })
    .await?;

    let stored_snapshots = store_chapter_snapshots(StoreChapterSnapshotsInput {
        state,
        db: &txn,
        profile: &profile,
        project_id,
        profile_name: &name,
        chapter_numbers: &selected_chapter_numbers,
        start_chapter,
        target_chapter,
        baseline_state: &baseline_state,
        behaviors: &behaviors,
        classification_suggestions: &classification_suggestions,
        deletion_candidates: &deletion_candidates,
        chapter_states: &chapter_states,
        implicit_review_decisions: &implicit_review_decisions,
        implicit_reviews_applied: write_summary.implicit_reviews_applied,
    })
    .await?;
    let mut state_indexes = Vec::with_capacity(stored_snapshots.states.len());
    for chapter_state in &stored_snapshots.states {
        let state_index = state
            .character
            .upsert_state_index_in(
                &txn,
                a4n_services::character::StateIndexUpsertInput {
                    profile_id: profile.id,
                    chapter_number: chapter_state.chapter_number,
                    state_summary: &chapter_state.state_summary,
                    key_attributes: chapter_state.attributes.clone(),
                    emotional_state: chapter_state.emotional_state.as_deref(),
                    current_location: chapter_state.current_location.as_deref(),
                    current_goal: chapter_state.current_goal.as_deref(),
                },
            )
            .await
            .map_err(|e| e.to_string())?;
        state_indexes.push(state_index);
    }

    let mut active: protagonist_profile::ActiveModel = profile.into();
    apply_profile_current_state(&mut active, stored_snapshots.target_attributes);
    active.last_synced_chapter = Set(target_chapter);
    active.updated_at = Set(chrono::Utc::now().naive_utc());
    active.update(&txn).await.map_err(|e| e.to_string())?;
    record_completed_generation_run(
        &txn,
        RecordCompletedGenerationRunInput {
            project_id,
            workflow: "character_profile_sync".to_string(),
            step_key: "sync_character_profile_from_chapters".to_string(),
            title: format!("角色档案同步: {name}"),
            model_kind: "llm".to_string(),
            provider: None,
            model: None,
            prompt_template_names_json: Some(generation_trace.prompt_template_names_json.clone()),
            prompt_template_hashes_json: None,
            rendered_messages_json: Some(generation_trace.rendered_messages_json.clone()),
            rendered_prompts_json: None,
            input_variables_json: Some(json!({
                "profile_id": profile_id,
                "character_name": &name,
                "last_synced_chapter_before_run": profile_last_synced_chapter,
                "start_chapter": start_chapter,
                "target_chapter": target_chapter,
                "clear_to_chapter": clear_to_chapter,
                "selected_chapter_numbers": &selected_chapter_numbers,
                "baseline_state": &baseline_state,
                "chapter_data": &chapter_data,
            })),
            source_refs_json: Some(json!({
                "profile": {
                    "id": profile_id,
                    "name": &name,
                },
                "chapters": selected_chapter_numbers
                    .iter()
                    .map(|chapter_number| json!({ "chapter_number": chapter_number }))
                    .collect::<Vec<_>>(),
            })),
            media_refs_json: None,
            request_json: Some(generation_trace.request_json.clone()),
            response_summary_json: Some(json!({
                "profile_id": profile_id,
                "character_name": &name,
                "response_chars": generation_trace.response_chars,
                "changes_applied": write_summary.changes_applied,
                "behaviors_recorded": write_summary.behaviors_recorded,
                "deletions_marked": write_summary.deletions_marked,
                "deletions_executed": write_summary.deletions_executed,
                "state_indexes_upserted": state_indexes.len(),
            })),
            targets: vec![GenerationRunTraceTarget {
                target_kind: "character_profile_record".to_string(),
                target_table: "protagonist_profile".to_string(),
                target_id: profile_id,
            }],
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;
    progress.report(
        9,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：角色数据已写入，正在同步角色 RAG"),
    )?;
    let (deleted_state_rag, mut state_rag_error) =
        match crate::workflow::delete_character_state_rag_by_ids(
            state,
            project_id,
            &stale_state_index_ids,
        )
        .await
        {
            Ok(deleted) => (deleted, None),
            Err(err) => {
                tracing::warn!(
                    profile_id,
                    project_id,
                    start_chapter,
                    clear_to_chapter,
                    state_ids = ?stale_state_index_ids,
                    error = %err,
                    "角色档案重同步后清理状态 RAG 失败"
                );
                (0, Some(err))
            }
        };
    let (deleted_attribute_event_rag, mut attribute_event_rag_error) =
        match crate::workflow::delete_character_attribute_event_rag_by_ids(
            state,
            project_id,
            &stale_attribute_event_ids,
        )
        .await
        {
            Ok(deleted) => (deleted, None),
            Err(err) => {
                tracing::warn!(
                    profile_id,
                    project_id,
                    start_chapter,
                    clear_to_chapter,
                    attribute_event_ids = ?stale_attribute_event_ids,
                    error = %err,
                    "角色档案重同步后清理属性事件 RAG 失败"
                );
                (0, Some(err))
            }
        };
    let mut state_rag_indexed_count = 0usize;
    let mut state_rag_index_errors = Vec::new();
    for state_index in &state_indexes {
        match crate::workflow::ingest_character_state_rag(state, project_id, state_index, &name)
            .await
        {
            Ok(()) => {
                state_rag_indexed_count += 1;
            }
            Err(err) => {
                tracing::warn!(
                    profile_id,
                    project_id,
                    state_id = state_index.id,
                    chapter_number = state_index.chapter_number,
                    error = %err,
                    "角色档案同步后状态 RAG 入库失败"
                );
                state_rag_index_errors.push(format!(
                    "第{}章状态 {}：{}",
                    state_index.chapter_number, state_index.id, err
                ));
            }
        }
    }
    if !state_rag_index_errors.is_empty() {
        let ingest_error = state_rag_index_errors.join("；");
        state_rag_error = Some(match state_rag_error {
            Some(delete_error) => format!("清理失败：{delete_error}；入库失败：{ingest_error}"),
            None => format!("入库失败：{ingest_error}"),
        });
    }
    let state_rag_indexed = state_rag_indexed_count == state_indexes.len();

    let mut attribute_event_rag_indexed_count = 0usize;
    let mut attribute_event_rag_index_errors = Vec::new();
    if !write_summary.attribute_event_ids.is_empty() {
        match attribute_events_by_ids(state, &write_summary.attribute_event_ids).await {
            Ok(attribute_events) => {
                let attribute_events_by_id = attribute_events
                    .into_iter()
                    .map(|event| (event.id, event))
                    .collect::<HashMap<_, _>>();
                for event_id in &write_summary.attribute_event_ids {
                    let Some(event) = attribute_events_by_id.get(event_id) else {
                        attribute_event_rag_index_errors
                            .push(format!("属性事件 {event_id}：数据库记录不存在"));
                        continue;
                    };
                    match crate::workflow::ingest_character_attribute_event_rag(
                        state, project_id, event, &name,
                    )
                    .await
                    {
                        Ok(()) => {
                            attribute_event_rag_indexed_count += 1;
                        }
                        Err(err) => {
                            tracing::warn!(
                                profile_id,
                                project_id,
                                attribute_event_id = event.id,
                                chapter_number = event.chapter_number,
                                error = %err,
                                "角色档案同步后属性事件 RAG 入库失败"
                            );
                            attribute_event_rag_index_errors.push(format!(
                                "第{}章属性事件 {}：{}",
                                event.chapter_number, event.id, err
                            ));
                        }
                    }
                }
            }
            Err(err) => {
                tracing::warn!(
                    profile_id,
                    project_id,
                    attribute_event_ids = ?write_summary.attribute_event_ids,
                    error = %err,
                    "角色档案同步后查询属性事件失败"
                );
                attribute_event_rag_index_errors.push(format!("查询属性事件失败：{err}"));
            }
        }
    }
    if !attribute_event_rag_index_errors.is_empty() {
        let ingest_error = attribute_event_rag_index_errors.join("；");
        attribute_event_rag_error = Some(match attribute_event_rag_error {
            Some(delete_error) => format!("清理失败：{delete_error}；入库失败：{ingest_error}"),
            None => format!("入库失败：{ingest_error}"),
        });
    }
    let attribute_event_rag_indexed =
        attribute_event_rag_indexed_count == write_summary.attribute_event_ids.len();

    progress.report(
        CHARACTER_PROFILE_SYNC_STEPS,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：角色同步完成"),
    )?;
    Ok(json!({
        "status": "synced",
        "name": name,
        "chapters_analyzed": selected_chapter_count,
        "sync_range": {
            "from": start_chapter,
            "to": target_chapter,
        },
        "synced_chapter": target_chapter,
        "changes_applied": write_summary.changes_applied,
        "behaviors_recorded": write_summary.behaviors_recorded,
        "classification_suggestions_applied": write_summary.classification_suggestions_applied,
        "deletions_marked": write_summary.deletions_marked,
        "deletions_executed": write_summary.deletions_executed,
        "deleted_deletion_marks": deletion_mark_rewind.deleted,
        "rewound_deletion_marks": deletion_mark_rewind.rewound,
        "implicit_reviews_checked": implicit_review_decisions.len(),
        "implicit_reviews_applied": write_summary.implicit_reviews_applied,
        "snapshot_hash": stored_snapshots.target_hash,
        "state_indexes_upserted": state_indexes.len(),
        "deleted_state_rag": deleted_state_rag,
        "state_rag_indexed_count": state_rag_indexed_count,
        "state_rag_indexed": state_rag_indexed,
        "state_rag_error": state_rag_error,
        "deleted_attribute_event_rag": deleted_attribute_event_rag,
        "attribute_event_rag_indexed_count": attribute_event_rag_indexed_count,
        "attribute_event_rag_indexed": attribute_event_rag_indexed,
        "attribute_event_rag_error": attribute_event_rag_error,
        "llm_trace_paths": llm_trace_paths,
    }))
}

async fn attribute_events_by_ids(
    state: &AppState,
    attribute_event_ids: &[i64],
) -> Result<Vec<attribute_event::Model>, String> {
    if attribute_event_ids.is_empty() {
        return Ok(Vec::new());
    }

    attribute_event::Entity::find()
        .filter(attribute_event::Column::Id.is_in(attribute_event_ids.to_vec()))
        .order_by_asc(attribute_event::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())
}
