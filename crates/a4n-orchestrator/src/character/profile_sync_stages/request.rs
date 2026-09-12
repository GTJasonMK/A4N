use std::collections::HashMap;

use a4n_error::AppError;
use a4n_llm::{
    context::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT,
        context_aware_system_user_json_llm_request,
    },
    provider::ChatRequest,
    trace::context_aware_chat_request_trace_json,
};
use a4n_prompt::{rendering::PromptRenderer, request::build_json_task_user_message};
use serde_json::{Value, json};

use crate::{
    character::character_profile_sync_stage_call_key,
    workflow::{self, workflow_checkpoint_contract_app_error},
};

use super::contracts::{CharacterProfileSyncStage, CharacterProfileSyncStageInput};

pub(super) const CHARACTER_PROFILE_SYNC_SESSION_PROMPT: &str = "character_sync_session";

pub(super) struct PreparedCharacterProfileSyncStageCall {
    pub(super) request: ChatRequest,
    pub(super) call_key: String,
    pub(super) input_fingerprint_json: Value,
}

pub(super) async fn build_character_profile_sync_stage_request(
    input: &CharacterProfileSyncStageInput,
    stage: CharacterProfileSyncStage,
    previous_steps: &Value,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<ChatRequest, AppError> {
    let stage_prompt = prompt_renderer
        .render_prompt(stage.prompt_name(), &HashMap::new(), Some(input.project_id))
        .await?;
    let system_prompt = prompt_renderer
        .render_prompt(
            CHARACTER_PROFILE_SYNC_SESSION_PROMPT,
            &HashMap::new(),
            Some(input.project_id),
        )
        .await?;
    let user_prompt = build_character_profile_sync_stage_user_prompt(stage, previous_steps)
        .map_err(AppError::InvalidArgument)?;
    let user_prompt = format!("[阶段指令]\n{}\n\n{user_prompt}", stage_prompt.trim());
    context_aware_system_user_json_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id: input.project_id,
            workflow_run_id: None,
            workflow: "character_sync".to_string(),
            step_key: stage.key().to_string(),
            scope_kind: "profile_sync".to_string(),
            pack_kind: PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "profile_id": input.profile_id,
                "profile_name": input.profile_name,
                "start_chapter": input.start_chapter,
                "target_chapter": input.target_chapter,
                "input_chapter_numbers": input.selected_chapter_numbers,
            })),
            config_json: json!({
                "ledger_version": "character_sync_context_v1",
            }),
            stable_context_json: character_profile_sync_stage_stable_context(input),
            dynamic_user_prompt: user_prompt,
            temperature: Some(stage.temperature()),
            call_key: Some(
                character_profile_sync_stage_call_key(
                    input.profile_id,
                    input.target_chapter,
                    stage.key(),
                )
                .map_err(AppError::InvalidArgument)?,
            ),
        },
    )
    .map_err(AppError::InvalidArgument)
}

pub(super) async fn prepare_character_profile_sync_stage_call(
    input: &CharacterProfileSyncStageInput,
    stage: CharacterProfileSyncStage,
    previous_steps: &Value,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<PreparedCharacterProfileSyncStageCall, AppError> {
    let mut request =
        build_character_profile_sync_stage_request(input, stage, previous_steps, prompt_renderer)
            .await?;
    let call_key =
        character_profile_sync_stage_call_key(input.profile_id, input.target_chapter, stage.key())
            .map_err(AppError::InvalidArgument)?;
    let request_json = context_aware_chat_request_trace_json(&request)?;
    let input_fingerprint_json = json!({
        "request": request_json,
        "stage": stage.key(),
        "profile_id": input.profile_id,
        "start_chapter": input.start_chapter,
        "target_chapter": input.target_chapter,
        "previous_steps": previous_steps,
    });
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = input.workflow_run_id;
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    Ok(PreparedCharacterProfileSyncStageCall {
        request,
        call_key,
        input_fingerprint_json,
    })
}

fn build_character_profile_sync_stage_user_prompt(
    stage: CharacterProfileSyncStage,
    previous_steps: &Value,
) -> Result<String, String> {
    let payload = json!({
        "context_refs": {
            "source": format!("稳定上下文包 {PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT}"),
        },
        "task": {
            "name": stage.key(),
            "instruction": stage.title(),
        },
        "output_contract": character_profile_sync_stage_contract(stage),
        "previous_steps": previous_steps,
    });
    build_json_task_user_message(
        stage.title(),
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|error| format!("角色同步阶段请求构造失败: {error}"))
}

fn character_profile_sync_stage_stable_context(input: &CharacterProfileSyncStageInput) -> Value {
    json!({
        "character": {
            "profile_id": input.profile_id,
            "profile_name": input.profile_name,
            "last_synced_chapter": input.last_synced_chapter,
        },
        "sync_range": {
            "start_chapter": input.start_chapter,
            "target_chapter": input.target_chapter,
            "input_chapter_numbers": input.selected_chapter_numbers,
        },
        "baseline_state": input.baseline_state,
        "base_attributes": input.base_attributes,
        "chapters": input.chapter_data,
    })
}

pub(super) fn character_profile_sync_stage_contract(stage: CharacterProfileSyncStage) -> Value {
    match stage {
        CharacterProfileSyncStage::EvidenceEvents => json!({
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
        CharacterProfileSyncStage::AttributeChanges => json!({
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
        CharacterProfileSyncStage::ChapterStates => json!({
            "format": "json_object",
            "required_root_field": "chapter_states",
            "item_required_fields": ["chapter_number", "state_summary", "current_location", "current_goal", "emotional_state"],
            "rules": [
                "chapter_states 必须覆盖 input_chapter_numbers 中每个章节且不能重复",
                "只输出章节结束时该角色的稳定状态，不要输出属性变化或行为记录",
                "current_location、current_goal、emotional_state 不明确时输出 null"
            ]
        }),
        CharacterProfileSyncStage::BehaviorRecords => json!({
            "format": "json_object",
            "required_root_field": "behaviors",
            "item_required_fields": ["chapter_number", "behavior_type", "behavior_description", "original_text_anchor_id", "behavior_tags", "related_characters", "impact_analysis"],
            "rules": [
                "只记录推动剧情、暴露性格、改变关系或影响目标的关键行为",
                "不要输出 classification_results 或 original_text",
                "impact_analysis 没有实质影响时输出空字符串"
            ]
        }),
        CharacterProfileSyncStage::DeletionCandidates => json!({
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
