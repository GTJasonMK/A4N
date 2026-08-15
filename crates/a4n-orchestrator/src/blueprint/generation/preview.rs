use std::collections::HashMap;

use super::contracts::BLUEPRINT_GENERATION_STAGES;
use a4n_core::blueprint::build_blueprint_generation_payload;
use a4n_core::error::AppError;
use a4n_prompt::blueprint_generation::{
    BlueprintGenerationPreviewStep, BlueprintStageRequestInput, build_blueprint_stage_request,
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

const BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME: &str =
    "blueprint + blueprint_stage_system + blueprint_stage_user";

pub(in crate::blueprint) async fn preview_generate_requests(
    project_id: i64,
    novel_title: &str,
    genre: &str,
    target_length: i32,
    prompt_renderer: &dyn PromptRenderer,
    variables: &HashMap<String, Value>,
) -> Result<Vec<BlueprintGenerationPreviewStep>, AppError> {
    let prompt_content = prompt_renderer
        .render_prompt("blueprint", variables, Some(project_id))
        .await?;
    let user_payload =
        build_blueprint_generation_payload(novel_title, genre, target_length, variables)?;
    let user_payload_json = serde_json::to_string_pretty(&user_payload)?;

    let spec = &BLUEPRINT_GENERATION_STAGES[0];
    let request = build_blueprint_stage_request(BlueprintStageRequestInput {
        prompt_renderer,
        project_id,
        base_prompt: &prompt_content,
        user_payload_json: &user_payload_json,
        stage_key: spec.key,
        stage_name: spec.title,
        output_schema: spec.output_schema,
        previous_context: None,
    })
    .await?;
    Ok(vec![ready_blueprint_preview_step(
        spec.key, spec.title, request,
    )])
}

fn ready_blueprint_preview_step(
    key: &str,
    title: &str,
    request: a4n_llm::provider::ChatRequest,
) -> BlueprintGenerationPreviewStep {
    BlueprintGenerationPreviewStep {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME.to_string(),
        state: "ready".to_string(),
        note: None,
        request: Some(request),
    }
}
