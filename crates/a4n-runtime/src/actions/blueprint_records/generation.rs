use a4n_api_contract::{
    blueprint::{BlueprintSummary, PreviewBlueprintGenerationInput, StartBlueprintGenerationInput},
    generation_tasks::GenerationTaskHandle,
};
use a4n_core::blueprint::BLUEPRINT_GENERATION_KIND;
use a4n_core::error::AppError;
use a4n_orchestrator::GenerationTaskAccessError;

use crate::AppState;
use crate::actions::request_preview::{ModelRequestPreview, chat_status_preview_for_state};

use super::projections::blueprint_summary;

pub async fn start_blueprint_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartBlueprintGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .orchestrator()
        .start_blueprint_generation_task(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        BLUEPRINT_GENERATION_KIND,
        move |handle| async move {
            let prompt_renderer = &worker_state;
            worker_state
                .orchestrator()
                .run_blueprint_generation_task(prompt_renderer, input, handle)
                .await
        },
    );
    Ok(start.handle)
}

pub async fn get_blueprint(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Option<BlueprintSummary>, AppError> {
    Ok(state
        .orchestrator()
        .novel_blueprint_for_user_validated(user_id, project_id)
        .await?
        .map(|(blueprint, needs_review)| blueprint_summary(&blueprint, needs_review)))
}

pub async fn preview_generate_blueprint_request(
    state: &AppState,
    user_id: i64,
    input: PreviewBlueprintGenerationInput,
) -> Result<ModelRequestPreview, AppError> {
    let variables = input.variables.unwrap_or_default().into_iter().collect();
    let prompt_renderer = state;
    let stage = state
        .orchestrator()
        .preview_blueprint_generation_request_for_user_validated(
            user_id,
            input.project_id,
            &input.title,
            &input.genre,
            input.target_length,
            prompt_renderer,
            &variables,
        )
        .await?;
    chat_status_preview_for_state(
        state,
        BLUEPRINT_GENERATION_KIND,
        "蓝图生成请求预览",
        [crate::actions::request_preview::ChatStatusPreviewStep::llm(
            stage.key,
            stage.title,
            Some(stage.prompt_name),
            "ready",
            None,
            Some(stage.request),
        )],
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}
