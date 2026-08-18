use a4n_api_contract::{
    blueprint::{PreviewBlueprintRefinementInput, StartBlueprintRefinementInput},
    generation_tasks::GenerationTaskHandle,
};
use a4n_core::{
    blueprint::{
        BLUEPRINT_REFINEMENT_PROMPTS, BLUEPRINT_REFINEMENT_TASK_KIND,
        BLUEPRINT_REFINEMENT_WORKFLOW_KIND,
    },
    error::AppError,
};
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
        .orchestrator()
        .start_blueprint_refinement_task(user_id, &input)
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
                .orchestrator()
                .run_blueprint_refinement_task(prompt_renderer, input, handle)
                .await
        },
    );
    Ok(start.handle)
}

pub async fn preview_refine_blueprint_request(
    state: &AppState,
    user_id: i64,
    input: PreviewBlueprintRefinementInput,
) -> Result<ModelRequestPreview, AppError> {
    let variables = input.variables.unwrap_or_default().into_iter().collect();
    let prompt_renderer = state;
    let request = state
        .orchestrator()
        .preview_blueprint_refine_request_for_user_validated(
            user_id,
            input.project_id,
            input.focus.as_deref(),
            prompt_renderer,
            &variables,
        )
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
