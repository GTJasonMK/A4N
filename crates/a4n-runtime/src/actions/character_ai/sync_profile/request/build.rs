use std::collections::HashMap;

use serde_json::{Value, json};

use crate::actions::character_ai::sync_types::CharacterSyncContext;
use crate::actions::llm_request::{
    ContextAwareJsonLlmRequestInput, PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT,
    context_aware_system_user_json_llm_request,
};
use crate::state::AppState;
use a4n_core::config::app_config::llm_token_profiles;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use a4n_prompt::request::build_json_task_user_message;

use super::super::contracts::CharacterSyncStage;
use super::super::stages::stage_temperature;
use super::contract::character_sync_stage_contract;

pub(in crate::actions::character_ai::sync_profile) async fn build_character_sync_stage_request(
    state: &AppState,
    project_id: i64,
    context: &CharacterSyncContext,
    stage: CharacterSyncStage,
    previous_steps: &Value,
) -> Result<ChatRequest, String> {
    let system_prompt = state
        .render_prompt(stage.prompt_name(), &HashMap::new(), Some(project_id))
        .await
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
                "input_chapter_numbers": context.selected_chapter_numbers})),
            config_json: json!({
                "ledger_version": "character_sync_context_v1",
                "prompt_name": stage.prompt_name(),
                "stage": stage.key()}),
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
            "instruction": stage.title()},
        "output_contract": character_sync_stage_contract(stage),
        "previous_steps": previous_steps});
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
            "last_synced_chapter": context.profile.last_synced_chapter},
        "sync_range": {
            "start_chapter": context.start_chapter,
            "target_chapter": context.target_chapter,
            "input_chapter_numbers": &context.selected_chapter_numbers},
        "baseline_state": &context.baseline_state,
        "base_attributes": &context.profile.base_attributes,
        "chapters": &context.chapter_data})
}
