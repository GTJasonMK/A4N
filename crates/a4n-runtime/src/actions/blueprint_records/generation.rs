use a4n_api_contract::workflow::WorkflowResumeRecord;
use a4n_api_contract::{
    blueprint::{BlueprintSummary, PreviewBlueprintGenerationInput, StartBlueprintGenerationInput},
    generation_tasks::GenerationTaskHandle,
};
use a4n_domain::project::BLUEPRINT_GENERATION_KIND;
use a4n_error::AppError;
use a4n_orchestrator::GenerationTaskAccessError;

use crate::AppState;
use crate::actions::request_preview::{ModelRequestPreview, chat_status_preview_for_state};

pub async fn start_blueprint_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartBlueprintGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .blueprint_generation_application()
        .start_generation(user_id, &input)
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
                .blueprint_generation_application()
                .run_generation(prompt_renderer, input, handle)
                .await
        },
    );
    Ok(start.handle)
}

pub async fn resume_blueprint_generation_workflow(
    state: &AppState,
    user_id: i64,
    run: &WorkflowResumeRecord,
) -> Result<(), String> {
    let input = state
        .blueprint_generation_application()
        .generation_input_from_workflow_run(run)
        .map_err(crate::actions::app_error_message)?;
    let start = state
        .blueprint_generation_application()
        .start_generation(user_id, &input)
        .await
        .map_err(|error| error.to_string())?;
    if let Err(error) = state
        .workflow_cancellation_application()
        .resume_with_active_task(run.id, &start.handle.task_id)
        .await
    {
        let cleanup = state
            .generation_task_mutation_application()
            .request_generation_cancel(user_id, &start.handle.task_id, start.handle.attempt)
            .await;
        return Err(match cleanup {
            Ok(_) => error.to_string(),
            Err(cleanup_error) => {
                format!("{error}; generation task 清理失败: {cleanup_error}")
            }
        });
    }
    if !start.should_run {
        return Ok(());
    }
    let worker_state = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        BLUEPRINT_GENERATION_KIND,
        move |handle| async move {
            let prompt_renderer = &worker_state;
            worker_state
                .blueprint_generation_application()
                .run_generation(prompt_renderer, input, handle)
                .await
        },
    );
    Ok(())
}

pub async fn get_blueprint(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Option<BlueprintSummary>, AppError> {
    state
        .blueprint_read_application()
        .summary(user_id, project_id)
        .await
}

pub async fn preview_generate_blueprint_request(
    state: &AppState,
    user_id: i64,
    input: PreviewBlueprintGenerationInput,
) -> Result<ModelRequestPreview, AppError> {
    let prompt_renderer = state;
    let stage = state
        .blueprint_request_preview_application()
        .preview_generation(user_id, input, prompt_renderer)
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
