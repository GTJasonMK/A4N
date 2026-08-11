use super::ReverseEngineeringTarget;
use super::contracts::{
    REVERSE_VARIABLES_ERROR, ReverseEngineerInput, ReverseEngineerTaskScope,
    ReverseProgressReporter,
};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use crate::{AppState, actions::access};
use a4n_core::error::AppError;
use a4n_orchestrator::import_reverse::ReverseEngineerApplicationInput;
use serde_json::Value;

pub async fn reverse_engineer_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
) -> Result<ReverseEngineerTaskScope, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    Ok(ReverseEngineerTaskScope {
        project_id: input.project_id,
        total: 3,
    })
}

pub async fn reverse_engineer(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    progress: ReverseProgressReporter<'_>,
    active_task_id: Option<&str>,
) -> Result<Value, AppError> {
    reverse_engineer_task_scope(state, user_id, input).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        REVERSE_VARIABLES_ERROR,
    )?;
    let prompt_renderer = state;
    state
        .orchestrator()
        .run_reverse_engineering_validated(ReverseEngineerApplicationInput {
            project_id: input.project_id,
            manual_sample: &input.content_sample,
            auto_context: input.auto_context,
            context_limit: input.context_limit,
            variables: &variables,
            target,
            prompt_renderer: prompt_renderer,
            progress,
            active_task_id,
        })
        .await
}

pub async fn confirm_reverse_engineering_candidate(
    state: &AppState,
    user_id: i64,
    input: &super::ConfirmReverseEngineeringCandidateInput,
) -> Result<Value, AppError> {
    let run = state
        .orchestrator()
        .workflow_run_model_by_id(input.workflow_run_id)
        .await?;
    let project_id =
        run.project_id
            .or(run.root_project_id)
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_reverse_candidate_project".to_string(),
                operation: "确认项目资料归纳候选".to_string(),
            })?;
    access::verify_project_editor(state, user_id, project_id).await?;
    state
        .orchestrator()
        .confirm_reverse_engineering_candidate_validated(input.workflow_run_id)
        .await
}
