use super::{ProjectProgressReadError, ProjectProgressStatus, ProjectProgressSummary};
use crate::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_RUNNING,
};
use a4n_core::models::generation::workflow_run;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ProgressCounts {
    pub(super) pending: i64,
    pub(super) running: i64,
    pub(super) completed: i64,
    pub(super) failed: i64,
    pub(super) cancelled: i64,
}

impl ProgressCounts {
    pub(super) fn add(&mut self, status: &str) {
        match status.trim() {
            "pending" => self.pending += 1,
            "running" | "generating" => self.running += 1,
            "completed" => self.completed += 1,
            "failed" => self.failed += 1,
            "cancelled" => self.cancelled += 1,
            _ => {}
        }
    }

    pub(super) fn total(self) -> i64 {
        self.pending + self.running + self.completed + self.failed + self.cancelled
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct PipelineProjectState {
    pub(super) running: bool,
    pub(super) failed: bool,
    pub(super) cancelled: bool,
    pub(super) message: Option<String>,
}

pub(super) async fn latest_pipeline_states_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
    workflow_kind: &str,
    query_operation: &'static str,
) -> Result<HashMap<i64, PipelineProjectState>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let runs = workflow_run::Entity::find()
        .distinct_on([workflow_run::Column::BusinessKey])
        .filter(workflow_run::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(workflow_run::Column::WorkflowKind.eq(workflow_kind))
        .order_by_asc(workflow_run::Column::BusinessKey)
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database(query_operation, error))?;
    let mut states = HashMap::<i64, PipelineProjectState>::new();
    for run in &runs {
        let Some(project_id) = run.project_id else {
            continue;
        };
        let current = pipeline_state_from_workflow_run(run);
        let state = states.entry(project_id).or_default();
        if current.failed {
            state.failed = true;
            if current.message.is_some() {
                state.message = current.message;
            }
        } else if current.running {
            state.running = true;
            if !state.failed && current.message.is_some() {
                state.message = current.message;
            }
        } else if current.cancelled {
            state.cancelled = true;
            if !state.failed && !state.running && current.message.is_some() {
                state.message = current.message;
            }
        }
    }
    Ok(states)
}

fn pipeline_state_from_workflow_run(run: &workflow_run::Model) -> PipelineProjectState {
    let mut pipeline = PipelineProjectState::default();
    match run.status.trim() {
        WORKFLOW_STATUS_RUNNING => pipeline.running = true,
        WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_INTERRUPTED => pipeline.failed = true,
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

pub(super) fn project_progress(
    kind: &str,
    stage: &str,
    label: &str,
    current: Option<i64>,
    total: Option<i64>,
    status: ProjectProgressStatus,
    detail: Option<String>,
) -> ProjectProgressSummary {
    ProjectProgressSummary {
        kind: kind.to_string(),
        stage: stage.to_string(),
        label: label.to_string(),
        current,
        total,
        status,
        detail,
    }
}

pub(super) fn progress_count_detail(
    label: &str,
    current: Option<i64>,
    total: Option<i64>,
) -> Option<String> {
    match (current, total) {
        (Some(current), Some(total)) if total > 0 => Some(format!("{label} {current}/{total}")),
        _ => None,
    }
}

pub(super) fn non_zero(value: i64) -> Option<i64> {
    (value > 0).then_some(value)
}
