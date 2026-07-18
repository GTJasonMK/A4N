use a4n_core::error::AppError;

use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::actions::template_variables::parse_template_variables_app_error as parse_template_variables;
use crate::prompt_renderer::RuntimePromptRenderer;

use super::contracts::{BlueprintSummary, RefineBlueprintInput};
use super::projections::blueprint_summary;

pub async fn refine_blueprint(
    state: &AppState,
    user_id: i64,
    input: RefineBlueprintInput,
) -> Result<BlueprintSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let blueprint = state
        .orchestrator
        .refine_blueprint(
            input.project_id,
            input.focus.as_deref(),
            &prompt_renderer,
            &variables,
        )
        .await?;
    Ok(blueprint_summary(&blueprint))
}

pub async fn preview_refine_blueprint_request(
    state: &AppState,
    user_id: i64,
    input: RefineBlueprintInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let request = state
        .orchestrator
        .preview_blueprint_refine_request(
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
