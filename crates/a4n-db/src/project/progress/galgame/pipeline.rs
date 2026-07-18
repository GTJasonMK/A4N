use super::{
    GALGAME_PIPELINE_WORKFLOW_KIND, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use crate::project::progress::ProjectProgressReadError;
use crate::project::progress::shared::PipelineProjectState;
use a4n_core::models::generation::workflow_run;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::Value;
use std::collections::HashMap;

pub(super) async fn latest_galgame_pipeline_states_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, PipelineProjectState>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let runs = workflow_run::Entity::find()
        .distinct_on([workflow_run::Column::ProjectId])
        .filter(workflow_run::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(workflow_run::Column::WorkflowKind.eq(GALGAME_PIPELINE_WORKFLOW_KIND))
        .order_by_asc(workflow_run::Column::ProjectId)
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database("查询 Galgame 最近管线失败", error))?;
    Ok(runs
        .iter()
        .filter_map(|run| {
            run.project_id
                .map(|project_id| (project_id, galgame_pipeline_state_from_workflow_run(run)))
        })
        .collect())
}

fn galgame_pipeline_state_from_workflow_run(run: &workflow_run::Model) -> PipelineProjectState {
    let mut pipeline = PipelineProjectState::default();
    match run.status.trim() {
        WORKFLOW_STATUS_RUNNING => pipeline.running = true,
        WORKFLOW_STATUS_FAILED => pipeline.failed = true,
        WORKFLOW_STATUS_CANCELLED => pipeline.cancelled = true,
        _ => {}
    }
    pipeline.message = run
        .error_message
        .as_deref()
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_string)
        .or_else(|| {
            run.progress_json
                .as_ref()
                .and_then(|progress| progress.get("message"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .map(str::to_string)
        });
    pipeline
}
