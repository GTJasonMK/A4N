use std::collections::HashMap;

use serde_json::{Value, json};

use a4n_domain::source_spans::text_anchor_prompt_values;
use a4n_error::AppError;
use a4n_llm_context::{
    ContextAwareJsonLlmRequestInput, PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT,
    context_aware_system_user_json_llm_request,
};
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_llm_trace::context_aware_chat_request_trace_json;
use a4n_prompt::{rendering::PromptRenderer, request::build_json_task_user_message};

use super::super::context::{
    character_profile_for_prompt, existing_scene_profile_for_prompt, object_profile_for_prompt,
    scene_profile_chapter_context,
};
use super::super::types::SceneProfileSyncRequestContext;

const SCENE_PROFILE_SYNC_SESSION_PROMPT: &str = "scene_profile_sync_session";

pub(in crate::scene_profile::application) async fn build_scene_profile_stage_request(
    prompt_renderer: &dyn PromptRenderer,
    context: &SceneProfileSyncRequestContext,
    workflow_run_id: Option<i64>,
    prompt_name: &str,
    instruction: &str,
    previous_stages: Value,
    output_contract: Value,
) -> Result<ChatRequest, AppError> {
    let stage_prompt = prompt_renderer
        .render_prompt(
            prompt_name,
            &HashMap::new(),
            Some(context.chapter.project_id),
        )
        .await?;
    let system_prompt = prompt_renderer
        .render_prompt(
            SCENE_PROFILE_SYNC_SESSION_PROMPT,
            &HashMap::new(),
            Some(context.chapter.project_id),
        )
        .await?;
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
    )?;
    let user_prompt = format!("[阶段指令]\n{}\n\n{user_prompt}", stage_prompt.trim());
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
                "text_anchor_count": context.text_anchors.len(),
                "existing_scene_count": context.existing_scene_profiles.len(),
                "character_count": context.character_profiles.len(),
                "object_count": context.object_profiles.len()})),
            config_json: json!({
                "ledger_version": "scene_profile_sync_context_v1",
                "chapter_id": context.chapter.id,
                "chapter_number": context.chapter.chapter_number}),
            stable_context_json: scene_profile_stage_stable_context(context),
            dynamic_user_prompt: user_prompt,
            temperature: Some(0.2),
            call_key: Some(format!(
                "scene_profile_sync:{}:{}",
                context.chapter.id, prompt_name
            )),
        },
    )
    .map_err(AppError::InvalidArgument)
}

fn scene_profile_stage_stable_context(context: &SceneProfileSyncRequestContext) -> Value {
    json!({
        "existing_scene_profiles": context.existing_scene_profiles.iter().map(existing_scene_profile_for_prompt).collect::<Vec<_>>(),
        "available_characters": context.character_profiles.iter().map(character_profile_for_prompt).collect::<Vec<_>>(),
        "available_object_profiles": context.object_profiles.iter().map(object_profile_for_prompt).collect::<Vec<_>>(),
        "chapter": scene_profile_chapter_context(&context.chapter),
        "text_anchors": text_anchor_prompt_values(std::slice::from_ref(&context.chapter), &context.text_anchors)})
}

pub(super) fn scene_profile_stage_step_key(
    context: &SceneProfileSyncRequestContext,
    stage: &str,
) -> String {
    format!(
        "scene_profile_sync.chapter.{}.{}",
        context.chapter.chapter_number, stage
    )
}

pub(super) fn scene_profile_stage_step_order(
    chapter_index: usize,
    stage_index: usize,
) -> Result<i32, AppError> {
    let order = chapter_index
        .checked_mul(10)
        .and_then(|value| value.checked_add(stage_index))
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| {
            AppError::InvalidArgument("场景档案同步阶段序号超出 usize 范围".to_string())
        })?;
    i32::try_from(order)
        .map_err(|_| AppError::InvalidArgument("场景档案同步阶段序号超出 i32 范围".to_string()))
}

pub(super) fn scene_profile_stage_fingerprint(
    call_key: &str,
    prompt_name: &str,
    root_field: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "scene_profile_sync",
        // 旧断点仅检查根数组，不能作为已通过字段校验的结果继续复用。
        "output_validation_version": 1,
        "call_key": call_key,
        "prompt_name": prompt_name,
        "root_field": root_field,
        "request": context_aware_chat_request_trace_json(request)?}))
}
