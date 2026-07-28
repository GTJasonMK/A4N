use a4n_api_contract::{
    blueprint::StartBlueprintRefinementInput, generation_tasks::GenerationTaskHandle,
};
use a4n_core::error::AppError;
use a4n_orchestrator::GenerationTaskAccessError;

use crate::AppState;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::actions::template_variables::parse_template_variables_app_error as parse_template_variables;
use crate::prompt_renderer::RuntimePromptRenderer;

use super::contracts::PreviewBlueprintRefinementInput;

pub async fn start_blueprint_refinement_typed(
    state: &AppState,
    user_id: i64,
    input: StartBlueprintRefinementInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .project_blueprint()
        .start_blueprint_refinement_task(user_id, input.clone())
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    let handle = start.handle.clone();
    state.spawn_generation_execution(handle.task_id.clone(), async move {
        let prompt_renderer = RuntimePromptRenderer::new(&worker_state);
        if let Err(error) = worker_state
            .project_blueprint()
            .run_blueprint_refinement_task(
                a4n_orchestrator::blueprint::BlueprintRefinementApplicationContext {
                    prompt_renderer: &prompt_renderer,
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
                "蓝图 refinement worker 失败"
            );
        }
    });
    Ok(start.handle)
}

pub async fn preview_refine_blueprint_request(
    state: &AppState,
    user_id: i64,
    input: PreviewBlueprintRefinementInput,
) -> Result<ModelRequestPreview, AppError> {
    let variables = parse_template_variables(&input.variables)?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let request = state
        .project_blueprint()
        .preview_blueprint_refine_request_for_user_validated(
            user_id,
            input.project_id,
            input.focus.as_deref(),
            &prompt_renderer,
            &variables,
        )
        .await?;
    chat_status_preview_for_state(
        state,
        "blueprint_refine",
        "蓝图优化请求预览",
        [ChatStatusPreviewStep::llm(
            "blueprint_refine",
            "优化蓝图",
            Some("blueprint_optimization".to_string()),
            "ready",
            None,
            Some(request),
        )],
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}
