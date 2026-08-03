use crate::workflow::{PersistedTaskInfo, WorkflowReadError, is_terminal_workflow_status};
use a4n_core::models::{generation::workflow_checkpoint, task::task_queue};
use a4n_queue::priority::{TaskInfo, TaskKind, TaskPriority, TaskProgress, TaskStatus};
use serde_json::Value;
use std::collections::BTreeSet;

pub fn task_info_from_row(row: task_queue::Model) -> Result<TaskInfo, WorkflowReadError> {
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

pub(in crate::workflow) fn generated_video_ids_for_active_checkpoints(
    checkpoints: &[workflow_checkpoint::Model],
) -> Vec<i64> {
    let mut video_ids = BTreeSet::new();
    for checkpoint in checkpoints {
        if is_terminal_workflow_status(&checkpoint.status) {
            continue;
        }
        if let Some(video_id) = checkpoint
            .artifacts_json
            .as_ref()
            .and_then(|value| value.get("generated_video_id"))
            .and_then(Value::as_i64)
        {
            video_ids.insert(video_id);
        }
        if let Some(video_id) = checkpoint
            .input_fingerprint_json
            .as_ref()
            .and_then(|value| value.get("generated_video_id"))
            .and_then(Value::as_i64)
        {
            video_ids.insert(video_id);
        }
    }
    video_ids.into_iter().collect()
}

pub(in crate::workflow) fn normalize_cancel_reason(reason: &str) -> String {
    let reason = reason.trim();
    if reason.is_empty() {
        "工作流已取消".to_string()
    } else {
        reason.to_string()
    }
}
