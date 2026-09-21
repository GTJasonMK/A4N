use crate::workflow::{PersistedTaskInfo, WorkflowReadError, is_terminal_workflow_status};
use a4n_db_models::models::task::task_queue;
use a4n_queue::priority::{TaskInfo, TaskKind, TaskPriority, TaskProgress, TaskStatus};
use sea_orm::FromQueryResult;
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn task_info_from_row(row: task_queue::Model) -> Result<TaskInfo, WorkflowReadError> {
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

pub(in crate::workflow) fn generated_video_ids_for_active_checkpoints(
    checkpoints: &[WorkflowCheckpointVideoRefs],
) -> Vec<i64> {
    let mut video_ids = BTreeSet::new();
    for checkpoint in checkpoints {
        if is_terminal_workflow_status(&checkpoint.status) {
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
