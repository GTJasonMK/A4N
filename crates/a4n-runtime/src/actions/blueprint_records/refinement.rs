use a4n_api_contract::workflow::WorkflowResumeRecord;
use a4n_api_contract::{
    blueprint::{PreviewBlueprintRefinementInput, StartBlueprintRefinementInput},
    generation_tasks::GenerationTaskHandle,
};
use a4n_domain::project::{
    BLUEPRINT_REFINEMENT_PROMPTS, BLUEPRINT_REFINEMENT_TASK_KIND,
    BLUEPRINT_REFINEMENT_WORKFLOW_KIND,
};
use a4n_error::AppError;
use a4n_orchestrator::GenerationTaskAccessError;

use crate::AppState;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};

pub async fn start_blueprint_refinement_typed(
    state: &AppState,
    user_id: i64,
    input: StartBlueprintRefinementInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .blueprint_refinement_application()
        .start_refinement(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        BLUEPRINT_REFINEMENT_TASK_KIND,
        move |handle| async move {
            let prompt_renderer = &worker_state;
            worker_state
                .blueprint_refinement_application()
                .run_refinement(prompt_renderer, input, handle)
                .await
        },
    );
    Ok(start.handle)
}

pub async fn resume_blueprint_refinement_workflow(
    state: &AppState,
    user_id: i64,
    run: &WorkflowResumeRecord,
) -> Result<(), String> {
    let input = state
        .blueprint_refinement_application()
        .refinement_input_from_workflow_run(run)
        .map_err(crate::actions::app_error_message)?;
    let start = state
        .blueprint_refinement_application()
        .start_refinement(user_id, &input)
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
        BLUEPRINT_REFINEMENT_TASK_KIND,
        move |handle| async move {
            let prompt_renderer = &worker_state;
            worker_state
                .blueprint_refinement_application()
                .run_refinement(prompt_renderer, input, handle)
                .await
        },
    );
    Ok(())
}

pub async fn preview_refine_blueprint_request(
    state: &AppState,
    user_id: i64,
    input: PreviewBlueprintRefinementInput,
) -> Result<ModelRequestPreview, AppError> {
    let prompt_renderer = state;
    let request = state
        .blueprint_request_preview_application()
        .preview_refinement(user_id, input, prompt_renderer)
        .await?;
    chat_status_preview_for_state(
        state,
        BLUEPRINT_REFINEMENT_WORKFLOW_KIND,
        "蓝图优化请求预览",
        [ChatStatusPreviewStep::llm(
            BLUEPRINT_REFINEMENT_WORKFLOW_KIND,
            "优化蓝图",
            Some(BLUEPRINT_REFINEMENT_PROMPTS.join(" + ")),
            "ready",
            None,
            Some(request),
        )],
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}
