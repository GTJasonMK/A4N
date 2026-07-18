use super::contracts::BLUEPRINT_GENERATION_STAGES;
use a4n_core::blueprint::blueprint_stage_previous_context;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::blueprint_generation::{BlueprintStageRequestInput, build_blueprint_stage_request};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

pub(super) async fn blueprint_stage_trace_requests(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    base_prompt: &str,
    user_payload_json: &str,
    outputs: &[Value],
) -> Result<Vec<(&'static str, ChatRequest)>, AppError> {
    let mut requests = Vec::with_capacity(BLUEPRINT_GENERATION_STAGES.len());
    for (stage_index, spec) in BLUEPRINT_GENERATION_STAGES.iter().enumerate() {
        let previous_context = blueprint_stage_previous_context(stage_index, outputs)?;
        let request = build_blueprint_stage_request(BlueprintStageRequestInput {
            prompt_renderer,
            project_id,
            base_prompt,
            user_payload_json,
            stage_key: spec.key,
            stage_name: spec.title,
            output_schema: spec.output_schema,
            previous_context: previous_context.as_ref(),
        })
        .await?;
        requests.push((spec.key, request));
    }
    Ok(requests)
}
