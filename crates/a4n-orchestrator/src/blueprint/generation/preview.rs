use std::collections::HashMap;

use super::contracts::BLUEPRINT_GENERATION_STAGES;
use a4n_blueprint_contract::BlueprintGenerationPreviewStep;
use a4n_domain::project::{
    BLUEPRINT_BASE_PROMPT, BLUEPRINT_GENERATION_PROMPTS, build_blueprint_generation_payload,
};
use a4n_error::AppError;
use a4n_prompt::blueprint_generation::{BlueprintStageRequestInput, build_blueprint_stage_request};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

pub(in crate::blueprint) async fn preview_generate_request(
    project_id: i64,
    novel_title: &str,
    genre: &str,
    target_length: i32,
    prompt_renderer: &dyn PromptRenderer,
    variables: &HashMap<String, Value>,
) -> Result<BlueprintGenerationPreviewStep, AppError> {
    let prompt_content = prompt_renderer
        .render_prompt(BLUEPRINT_BASE_PROMPT, variables, Some(project_id))
        .await?;
    let user_payload =
        build_blueprint_generation_payload(novel_title, genre, target_length, variables)
            .map_err(AppError::InvalidArgument)?;
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
    Ok(BlueprintGenerationPreviewStep {
        key: spec.key.to_string(),
        title: spec.title.to_string(),
        prompt_name: BLUEPRINT_GENERATION_PROMPTS.join(" + "),
        request,
    })
}
