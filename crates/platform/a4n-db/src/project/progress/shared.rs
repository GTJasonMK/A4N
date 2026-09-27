use super::{ProjectProgressReadError, ProjectProgressSummary};
use crate::workflow::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WORKFLOW_STATUS_SUPERSEDED,
};
use a4n_db_models::models::{chapter::chapter_video_prompt, generation::workflow_run};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
    sea_query::{Expr, extension::postgres::PgExpr},
};
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
    pub(super) fn add(
        &mut self,
        table: &'static str,
        record_id: i64,
        status: &str,
    ) -> Result<(), ProjectProgressReadError> {
        let valid = match table {
            "chapter_manga_prompt" => matches!(
                status,
                "pending" | "generating" | "completed" | "failed" | "cancelled"
            ),
            "chapter_video_prompt" => matches!(
                status,
                chapter_video_prompt::STATUS_PENDING
                    | chapter_video_prompt::STATUS_RUNNING
                    | chapter_video_prompt::STATUS_COMPLETED
                    | chapter_video_prompt::STATUS_FAILED
                    | chapter_video_prompt::STATUS_INTERRUPTED
                    | chapter_video_prompt::STATUS_CANCELLED
            ),
            "generated_video" => matches!(
                status,
                a4n_domain::video_workflow::VIDEO_STATUS_PENDING
                    | a4n_domain::video_workflow::VIDEO_STATUS_RUNNING
                    | a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
                    | a4n_domain::video_workflow::VIDEO_STATUS_FAILED
                    | a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED
                    | a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED
            ),
            _ => false,
        };
        if !valid {
            return Err(ProjectProgressReadError::invalid_persisted_status(
                table,
                record_id,
                status.to_string(),
            ));
        }
        match status {
            "pending" => self.pending += 1,
            "running" | "generating" => self.running += 1,
            "completed" => self.completed += 1,
            "failed" | "interrupted" => self.failed += 1,
            "cancelled" => self.cancelled += 1,
            _ => unreachable!("status vocabulary checked above"),
        }
        Ok(())
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

#[derive(Debug, FromQueryResult)]
struct PipelineWorkflowProgressRow {
    id: i64,
    project_id: Option<i64>,
    status: String,
    progress_message: Option<Value>,
    error_message: Option<String>,
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
        .select_only()
        .column(workflow_run::Column::Id)
        .column(workflow_run::Column::ProjectId)
        .column(workflow_run::Column::Status)
        .column_as(
            Expr::col(workflow_run::Column::ProgressJson).get_json_field("message"),
            "progress_message",
        )
        .column(workflow_run::Column::ErrorMessage)
        .distinct_on([workflow_run::Column::BusinessKey])
        .filter(workflow_run::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(workflow_run::Column::WorkflowKind.eq(workflow_kind))
        .order_by_asc(workflow_run::Column::BusinessKey)
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .into_model::<PipelineWorkflowProgressRow>()
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database(query_operation, error))?;
    let mut states = HashMap::<i64, PipelineProjectState>::new();
    for run in &runs {
        let project_id = workflow_project_id(run)?;
        let current = pipeline_state_from_workflow_run(run)?;
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

fn workflow_project_id(run: &PipelineWorkflowProgressRow) -> Result<i64, ProjectProgressReadError> {
    run.project_id
        .ok_or(ProjectProgressReadError::InvalidPersistedReference {
            table: "workflow_run",
            record_id: run.id,
            field: "project_id",
        })
}

fn pipeline_state_from_workflow_run(
    run: &PipelineWorkflowProgressRow,
) -> Result<PipelineProjectState, ProjectProgressReadError> {
    let mut pipeline = PipelineProjectState::default();
    match run.status.as_str() {
        WORKFLOW_STATUS_RUNNING => pipeline.running = true,
        WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_INTERRUPTED => pipeline.failed = true,
        WORKFLOW_STATUS_CANCELLED => pipeline.cancelled = true,
        WORKFLOW_STATUS_COMPLETED | WORKFLOW_STATUS_SUPERSEDED => {}
        _ => {
            return Err(ProjectProgressReadError::invalid_persisted_status(
                "workflow_run",
                run.id,
                run.status.clone(),
            ));
        }
    }
    pipeline.message = run
        .error_message
        .as_deref()
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_string)
        .or_else(|| {
            run.progress_message
                .as_ref()
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .map(str::to_string)
        });
    Ok(pipeline)
}

pub(super) fn project_progress(label: &str, detail: Option<String>) -> ProjectProgressSummary {
    ProjectProgressSummary {
        label: label.to_string(),
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

#[cfg(test)]
mod database_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn workflow_run(status: &str) -> PipelineWorkflowProgressRow {
        PipelineWorkflowProgressRow {
            id: 9,
            project_id: Some(3),
            status: status.to_string(),
            progress_message: None,
            error_message: None,
        }
    }

    #[test]
    fn pipeline_progress_rejects_unknown_persisted_status() {
        let mut counts = ProgressCounts::default();
        assert!(matches!(
            counts.add("generated_video", 4, "legacy"),
            Err(ProjectProgressReadError::InvalidPersistedStatus {
                table: "generated_video",
                record_id: 4,
                value,
            }) if value == "legacy"
        ));
        assert!(matches!(
            counts.add("chapter_manga_prompt", 5, "interrupted"),
            Err(ProjectProgressReadError::InvalidPersistedStatus {
                table: "chapter_manga_prompt",
                record_id: 5,
                value,
            }) if value == "interrupted"
        ));

        assert!(matches!(
            pipeline_state_from_workflow_run(&workflow_run("legacy")),
            Err(ProjectProgressReadError::InvalidPersistedStatus {
                table: "workflow_run",
                record_id: 9,
                value,
            }) if value == "legacy"
        ));

        let mut missing_project = workflow_run(WORKFLOW_STATUS_COMPLETED);
        missing_project.project_id = None;
        assert!(matches!(
            workflow_project_id(&missing_project),
            Err(ProjectProgressReadError::InvalidPersistedReference {
                table: "workflow_run",
                record_id: 9,
                field: "project_id",
            })
        ));
    }
}
