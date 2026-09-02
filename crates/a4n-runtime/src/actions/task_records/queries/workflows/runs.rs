use crate::{
    AppState,
    actions::{access, admin_records, app_error_message},
};
use a4n_api_contract::workflow::{
    GetWorkflowRunInput, ListWorkflowRunsInput, WorkflowRunDetailSummary, WorkflowRunSummary,
};
use a4n_db::workflow::{
    GetWorkflowRunInput as DbGetWorkflowRunInput, ListWorkflowRunsInput as DbListWorkflowRunsInput,
};
use a4n_orchestrator::workflow::{require_positive_workflow_run_id, validate_optional_project_id};

use super::super::super::{
    access::{project_ids_visible_to_user, verify_workflow_run_summary_access},
    projections::{workflow_run_detail, workflow_run_summary},
};

pub async fn list_workflow_runs_for_user(
    state: &AppState,
    user_id: i64,
    input: ListWorkflowRunsInput,
) -> Result<Vec<WorkflowRunSummary>, String> {
    validate_optional_project_id(input.project_id)?;
    if !(1..=200).contains(&input.limit) {
        return Err("limit 必须在 1 到 200 之间".to_string());
    }
    let project_scoped = input.project_id.is_some();
    if let Some(project_id) = input.project_id {
        access::verify_project_editor_string(state, user_id, project_id).await?;
    }
    let db_input = DbListWorkflowRunsInput {
        project_id: input.project_id,
        chapter_id: input.chapter_id,
        chapter_ids: None,
        workflow_kind: input.workflow_kind,
        status: input.status,
        resumable_only: Some(input.resumable_only),
        limit: Some(input.limit),
    };
    let runs = state
        .orchestrator()
        .workflow_runs(db_input)
        .await
        .map_err(app_error_message)?;
    if project_scoped {
        return Ok(runs.into_iter().map(workflow_run_summary).collect());
    }
    let project_ids = runs
        .iter()
        .filter_map(|run| run.project_id.or(run.root_project_id))
        .collect::<Vec<_>>();
    let visible_project_ids = project_ids_visible_to_user(state, user_id, &project_ids).await?;
    let unscoped_visible = if runs
        .iter()
        .any(|run| run.project_id.is_none() && run.root_project_id.is_none())
    {
        admin_records::is_active_admin_user(state, user_id)
            .await
            .map_err(|err| err.to_string())?
    } else {
        false
    };
    let mut visible = Vec::new();
    for run in runs {
        let is_visible = match run.project_id.or(run.root_project_id) {
            Some(project_id) => visible_project_ids.contains(&project_id),
            None => unscoped_visible,
        };
        if is_visible {
            visible.push(workflow_run_summary(run));
        }
    }
    Ok(visible)
}

pub async fn get_workflow_run_detail_for_user(
    state: &AppState,
    user_id: i64,
    input: GetWorkflowRunInput,
) -> Result<WorkflowRunDetailSummary, String> {
    require_positive_workflow_run_id(input.workflow_run_id)?;
    let db_input = DbGetWorkflowRunInput {
        workflow_run_id: input.workflow_run_id,
        include_generation_runs: input.include_generation_runs,
    };
    let detail = state
        .orchestrator()
        .workflow_run_detail(db_input)
        .await
        .map_err(app_error_message)?;
    verify_workflow_run_summary_access(state, user_id, &detail.run).await?;
    Ok(workflow_run_detail(detail))
}
