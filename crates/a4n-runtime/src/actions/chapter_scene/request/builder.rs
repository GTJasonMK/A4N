use super::{
    context::scene_generation_stable_context, prompt::build_scene_generation_manifest_user_prompt,
};
use crate::actions::llm_request::{
    ContextAwareJsonLlmRequestInput, PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT,
    context_aware_system_user_llm_request,
};
use a4n_core::{config::app_config::llm_token_profiles, error::AppError};
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

pub(super) fn build_scene_generation_request(
    system_prompt: String,
    project_id: i64,
    chapter_number: i32,
    title: &str,
    summary: &str,
    generation_context: Option<&Value>,
    scene_number: i32,
    scene_prompt: &str,
) -> Result<ChatRequest, AppError> {
    let user_prompt = build_scene_generation_manifest_user_prompt(scene_number, scene_prompt)?;
    context_aware_system_user_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id,
            workflow_run_id: None,
            workflow: "chapter_scene_generation".to_string(),
            step_key: "scene_generation".to_string(),
            scope_kind: "chapter".to_string(),
            pack_kind: PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "chapter_number": chapter_number,
                "chapter_title": title,
                "scene_number": scene_number})),
            config_json: json!({
                "ledger_version": "chapter_scene_generation_context_v1",
                "prompt_name": "chapter_scene_generation",
                "chapter_number": chapter_number}),
            stable_context_json: scene_generation_stable_context(
                chapter_number,
                title,
                summary,
                generation_context,
            ),
            dynamic_user_prompt: user_prompt,
            temperature: Some(0.8),
            max_tokens_profile: llm_token_profiles::STANDARD.to_string(),
            call_key: Some(format!(
                "chapter_scene_generation:{chapter_number}:{scene_number}"
            )),
        },
    )
    .map_err(AppError::InvalidArgument)
}
