use super::super::contracts::{GALGAME_PIPELINE_WORKFLOW_KIND, GalgamePipelineStatusSummary};
use super::super::status::get_galgame_pipeline_status;
use crate::AppState;
use crate::actions::access;
use crate::actions::galgame_records::{GalgameProjectInput, project_record_by_id};
use a4n_orchestrator::galgame::PROJECT_KIND_GALGAME;
use a4n_orchestrator::workflow::{
    ListWorkflowRunsInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_RUNNING,
};

pub async fn cancel_galgame_pipeline(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let project = project_record_by_id(state, input.project_id).await?;
    if project.project_kind != PROJECT_KIND_GALGAME {
        return Err(format!("项目不是 Galgame 派生项目: {}", project.id));
    }
    let Some(run) = state
        .orchestrator
        .workflow_run_models(ListWorkflowRunsInput {
            project_id: Some(project.id),
            workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
            status: Some(WORKFLOW_STATUS_RUNNING.to_string()),
            resumable_only: None,
            limit: Some(1),
        })
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .next()
    else {
        return Err("没有正在运行的 Galgame 管线可取消".to_string());
    };

    state
        .orchestrator
        .mark_workflow_status(
            run.id,
            WORKFLOW_STATUS_CANCELLED,
            true,
            Some("用户请求取消 Galgame 管线"),
        )
        .await
        .map_err(|err| err.to_string())?;
    get_galgame_pipeline_status(
        state,
        user_id,
        GalgameProjectInput {
            project_id: project.id,
        },
    )
    .await
}
