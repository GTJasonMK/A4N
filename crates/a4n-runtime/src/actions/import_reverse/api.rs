use super::context::{apply_reverse_engineering_result, reverse_content_sample};
use super::contracts::{
    REVERSE_VARIABLES_ERROR, ReverseEngineerInput, ReverseEngineerTaskScope,
    ReverseProgressReporter,
};
use super::request::reverse_engineer_with_target;
use super::target::ReverseEngineeringTarget;
use super::workflow::{
    complete_reverse_workflow_run, fail_reverse_workflow_run, start_reverse_workflow_run,
};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use crate::actions::workflow_status_write_app_error;
use crate::{AppState, actions::access};
use a4n_core::error::AppError;
use serde_json::{Value, json};

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
    let scope = reverse_engineer_task_scope(state, user_id, input, apply).await?;
    progress(0, scope.total, target.preparing_context_message())?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        REVERSE_VARIABLES_ERROR,
    )?;
    let content_sample = reverse_content_sample(state, input, target).await?;
    let workflow_run_id =
        start_reverse_workflow_run(state, input, target, apply, &content_sample, &variables)
            .await?;

    progress(1, scope.total, target.calling_model_message())?;
    let result = match reverse_engineer_with_target(
        state,
        input.project_id,
        target,
        &content_sample,
        &variables,
        Some(workflow_run_id),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            if let Err(status_error) =
                fail_reverse_workflow_run(state, workflow_run_id, &error).await
            {
                return Err(workflow_status_write_app_error(&error, &status_error));
            }
            return Err(error);
        }
    };
    progress(2, scope.total, target.organizing_message())?;
    if !apply {
        complete_reverse_workflow_run(state, workflow_run_id).await?;
        return Ok(result);
    }

    progress(3, scope.total, target.applying_message())?;
    let applied =
        match apply_reverse_engineering_result(state, input.project_id, target, &result).await {
            Ok(applied) => applied,
            Err(error) => {
                if let Err(status_error) =
                    fail_reverse_workflow_run(state, workflow_run_id, &error).await
                {
                    return Err(workflow_status_write_app_error(&error, &status_error));
                }
                return Err(error);
            }
        };
    progress(4, scope.total, target.applied_message())?;
    complete_reverse_workflow_run(state, workflow_run_id).await?;
    Ok(json!({
        "result": result,
        "applied": applied
    }))
}
