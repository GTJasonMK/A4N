use crate::workflow::{PersistedTaskInfo, WorkflowReadError, is_terminal_workflow_status};
use a4n_db_models::models::task::task_queue;
use a4n_queue::priority::{TaskInfo, TaskKind, TaskPriority, TaskProgress, TaskStatus};
use sea_orm::FromQueryResult;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, FromQueryResult)]
pub(crate) struct TaskInfoProjectionRow {
    pub(crate) id: String,
    pub(crate) task_type: String,
    pub(crate) priority: String,
    pub(crate) status: String,
    pub(crate) project_id: Option<i64>,
    pub(crate) root_project_id: Option<i64>,
    pub(crate) source_snapshot_id: Option<i64>,
    pub(crate) task_scope: Option<String>,
    pub(crate) workflow_run_id: Option<i64>,
    pub(crate) progress: Option<Value>,
    pub(crate) error_message: Option<String>,
    pub(crate) created_at: chrono::NaiveDateTime,
    pub(crate) started_at: Option<chrono::NaiveDateTime>,
    pub(crate) completed_at: Option<chrono::NaiveDateTime>,
}

impl From<task_queue::Model> for TaskInfoProjectionRow {
    fn from(row: task_queue::Model) -> Self {
        Self {
            id: row.id,
            task_type: row.task_type,
            priority: row.priority,
            status: row.status,
            project_id: row.project_id,
            root_project_id: row.root_project_id,
            source_snapshot_id: row.source_snapshot_id,
            task_scope: row.task_scope,
            workflow_run_id: row.workflow_run_id,
            progress: row.progress,
            error_message: row.error_message,
            created_at: row.created_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
        }
    }
}

pub(crate) fn task_info_from_row(row: task_queue::Model) -> Result<TaskInfo, WorkflowReadError> {
    task_info_from_fields(row.into())
}

fn task_info_from_fields(row: TaskInfoProjectionRow) -> Result<TaskInfo, WorkflowReadError> {
    Ok(TaskInfo {
        id: row.id,
        task_type: TaskKind::parse_storage(&row.task_type)
            .map_err(|error| WorkflowReadError::InvalidData(error.to_string()))?,
        priority: TaskPriority::parse_storage(&row.priority)
            .map_err(|error| WorkflowReadError::InvalidData(error.to_string()))?,
        status: TaskStatus::parse_storage(&row.status, row.error_message.as_deref())
            .map_err(|error| WorkflowReadError::InvalidData(error.to_string()))?,
        project_id: row.project_id,
        root_project_id: row.root_project_id,
        source_snapshot_id: row.source_snapshot_id,
        task_scope: row.task_scope,
        progress: row
            .progress
            .map(serde_json::from_value::<TaskProgress>)
            .transpose()
            .map_err(|error| WorkflowReadError::InvalidData(error.to_string()))?,
        created_at: row.created_at,
        started_at: row.started_at,
        completed_at: row.completed_at,
    })
}

pub(crate) fn persisted_task_info_from_projection_row(
    row: TaskInfoProjectionRow,
) -> Result<PersistedTaskInfo, WorkflowReadError> {
    let workflow_run_id = row.workflow_run_id;
    let task = task_info_from_fields(row)?;
    Ok(PersistedTaskInfo {
        task,
        workflow_run_id,
    })
}

#[cfg(test)]
pub(in crate::workflow) fn persisted_task_info_from_row(
    row: task_queue::Model,
) -> Result<PersistedTaskInfo, WorkflowReadError> {
    let workflow_run_id = row.workflow_run_id;
    let task = task_info_from_row(row)?;
    Ok(PersistedTaskInfo {
        task,
        workflow_run_id,
    })
}

#[derive(Debug, FromQueryResult)]
pub(in crate::workflow) struct WorkflowCheckpointVideoRefs {
    pub(in crate::workflow) status: String,
    pub(in crate::workflow) artifact_generated_video_id: Option<Value>,
    pub(in crate::workflow) input_generated_video_id: Option<Value>,
}

pub(in crate::workflow) fn generated_video_ids_for_cancellation_checkpoints(
    checkpoints: &[WorkflowCheckpointVideoRefs],
) -> Vec<i64> {
    let mut video_ids = BTreeSet::new();
    for checkpoint in checkpoints {
        if is_terminal_workflow_status(&checkpoint.status)
            && checkpoint.status != crate::workflow::WORKFLOW_STATUS_CANCELLED
        {
            continue;
        }
        if let Some(video_id) = checkpoint
            .artifact_generated_video_id
            .as_ref()
            .and_then(Value::as_i64)
        {
            video_ids.insert(video_id);
        }
        if let Some(video_id) = checkpoint
            .input_generated_video_id
            .as_ref()
            .and_then(Value::as_i64)
        {
            video_ids.insert(video_id);
        }
    }
    video_ids.into_iter().collect()
}
