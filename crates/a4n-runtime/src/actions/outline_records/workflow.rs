use super::*;

pub(super) async fn start_outline_workflow_run(
    state: &AppState,
    project_id: i64,
    workflow_kind: &str,
    mode: &str,
    current_step_key: &str,
    task_id: Option<String>,
    input_fingerprint_json: Value,
) -> Result<i64, AppError> {
    let run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!("{workflow_kind}:project:{project_id}:{mode}"),
            workflow_kind: workflow_kind.to_string(),
            project_id: Some(project_id),
            root_project_id: Some(project_id),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(input_fingerprint_json),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(current_step_key.to_string()),
            active_task_id: task_id,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    Ok(run.id)
}

pub(super) async fn complete_outline_workflow_run(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<(), AppError> {
    state
        .orchestrator
        .mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await?;
    Ok(())
}

pub(super) async fn fail_outline_workflow_run(
    state: &AppState,
    workflow_run_id: i64,
    err: &AppError,
) -> Result<(), AppError> {
    state
        .orchestrator
        .mark_workflow_status(
            workflow_run_id,
            WORKFLOW_STATUS_FAILED,
            true,
            Some(&err.to_string()),
        )
        .await?;
    Ok(())
}

pub(super) async fn unwrap_outline_workflow_result<T>(
    state: &AppState,
    workflow_run_id: i64,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    match result {
        Ok(value) => Ok(value),
        Err(err) => {
            if let Err(status_error) = fail_outline_workflow_run(state, workflow_run_id, &err).await
            {
                return Err(crate::actions::workflow_status_write_app_error(
                    &err,
                    &status_error,
                ));
            }
            Err(err)
        }
    }
}

pub(super) async fn finish_outline_postprocess(
    state: &AppState,
    workflow_run_id: i64,
    result: Result<(), AppError>,
) -> Result<(), AppError> {
    match result {
        Ok(()) => complete_outline_workflow_run(state, workflow_run_id).await,
        Err(err) => {
            if let Err(status_error) = fail_outline_workflow_run(state, workflow_run_id, &err).await
            {
                return Err(crate::actions::workflow_status_write_app_error(
                    &err,
                    &status_error,
                ));
            }
            Err(err)
        }
    }
}
