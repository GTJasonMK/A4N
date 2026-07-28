use super::ReverseEngineeringTarget;
use super::contracts::{
    REVERSE_VARIABLES_ERROR, ReverseEngineerInput, ReverseEngineerTaskScope,
    ReverseProgressReporter,
};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use crate::prompt_renderer::RuntimePromptRenderer;
use crate::{AppState, actions::access};
use a4n_core::error::AppError;
use a4n_orchestrator::import_reverse::ReverseEngineerApplicationInput;
use serde_json::Value;

pub async fn reverse_engineer_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    apply: bool,
) -> Result<ReverseEngineerTaskScope, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    Ok(ReverseEngineerTaskScope {
        project_id: input.project_id,
        total: if apply { 4 } else { 3 },
    })
}

pub async fn reverse_engineer(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    progress: ReverseProgressReporter<'_>,
) -> Result<Value, AppError> {
    run_reverse_engineer(state, user_id, input, target, false, progress).await
}

pub async fn reverse_engineer_and_apply(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    progress: ReverseProgressReporter<'_>,
) -> Result<Value, AppError> {
    run_reverse_engineer(state, user_id, input, target, true, progress).await
}

async fn run_reverse_engineer(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    apply: bool,
    progress: ReverseProgressReporter<'_>,
) -> Result<Value, AppError> {
    reverse_engineer_task_scope(state, user_id, input, apply).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        REVERSE_VARIABLES_ERROR,
    )?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .import_reverse()
        .run_reverse_engineering_validated(ReverseEngineerApplicationInput {
            project_id: input.project_id,
            manual_sample: &input.content_sample,
            auto_context: input.auto_context,
            context_limit: input.context_limit,
            variables: &variables,
            target,
            apply,
            prompt_renderer: &prompt_renderer,
            progress,
        })
        .await
}
