use super::contracts::{ConverseInput, ConverseStreamInput, INSPIRATION_VARIABLES_ERROR};
use super::request::build_inspiration_request_for_runtime;
use super::rules::inspiration_stream_task_id;
use super::workflow::{finalize_inspiration_workflow, start_inspiration_workflow};
use crate::AppState;
use crate::actions::access;
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use a4n_core::error::AppError;
use a4n_orchestrator::inspiration::InspirationStreamInput;

pub async fn converse_inspiration(
    state: &AppState,
    user_id: i64,
    input: ConverseInput,
) -> Result<String, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        INSPIRATION_VARIABLES_ERROR,
    )?;
    let request = build_inspiration_request_for_runtime(
        state,
        input.project_id,
        &input.user_input,
        &variables,
    )
    .await?;
    let workflow_run_id = start_inspiration_workflow(
        state,
        input.project_id,
        "non_stream",
        input.variables.as_ref(),
        &request,
        None,
    )
    .await?;
    let result = state
        .orchestrator
        .converse_inspiration_with_request(
            input.project_id,
            &input.user_input,
            request,
            Some(workflow_run_id),
        )
        .await;
    finalize_inspiration_workflow(state, workflow_run_id, &result).await?;
    result
}

pub async fn converse_inspiration_stream<F>(
    state: &AppState,
    user_id: i64,
    input: ConverseStreamInput,
    on_chunk: F,
) -> Result<String, AppError>
where
    F: FnMut(&str, u32),
{
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        INSPIRATION_VARIABLES_ERROR,
    )?;
    let request =
        build_inspiration_request_for_runtime(state, input.project_id, &input.message, &variables)
            .await?;
    let active_task_id = input
        .task_id
        .as_ref()
        .map(|task_id| inspiration_stream_task_id(input.project_id, Some(task_id.clone())))
        .transpose()?;
    let workflow_run_id = start_inspiration_workflow(
        state,
        input.project_id,
        "stream",
        input.variables.as_ref(),
        &request,
        active_task_id,
    )
    .await?;
    let result = state
        .orchestrator
        .converse_inspiration_stream_with_request(InspirationStreamInput {
            project_id: input.project_id,
            user_input: &input.message,
            request,
            workflow_run_id: Some(workflow_run_id),
            on_chunk,
        })
        .await;
    finalize_inspiration_workflow(state, workflow_run_id, &result).await?;
    result
}
