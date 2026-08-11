use a4n_api_contract::{
    blueprint::StartBlueprintGenerationInput, generation_tasks::GenerationTaskHandle,
};
use a4n_core::error::AppError;
use a4n_orchestrator::GenerationTaskAccessError;

use crate::AppState;
use crate::actions::request_preview::{ModelRequestPreview, chat_status_preview_for_state};
use crate::actions::template_variables::parse_template_variables_app_error as parse_template_variables;

use super::contracts::{BlueprintSummary, PreviewBlueprintGenerationInput};
use super::projections::blueprint_summary;

pub async fn start_blueprint_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartBlueprintGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .orchestrator()
        .start_blueprint_generation_task(user_id, input.clone())
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    let handle = start.handle.clone();
    state.spawn_generation_execution(handle.task_id.clone(), async move {
        let prompt_renderer = &worker_state;
        if let Err(error) = worker_state
            .orchestrator()
            .run_blueprint_generation_task(
                a4n_orchestrator::blueprint::BlueprintGenerationApplicationContext {
                    prompt_renderer: prompt_renderer,
                },
                user_id,
                input,
                handle.clone(),
            )
            .await
        {
            tracing::error!(
                task_id = %handle.task_id,
                attempt = handle.attempt,
                error = %error,
                "蓝图 generation worker 失败"
            );
        }
    });
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
    let variables = parse_template_variables(&input.variables)?;
    let prompt_renderer = state;
    let stages = state
        .orchestrator()
        .preview_blueprint_generation_requests_for_user_validated(
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
        "blueprint_generation",
        "蓝图生成请求预览",
        stages
            .into_iter()
            .map(super::projections::blueprint_preview_step),
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}
