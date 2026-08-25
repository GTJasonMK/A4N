use crate::{AppState, actions::app_error_message};
use a4n_api_contract::workflow::{
    WorkflowProgressEvent, WorkflowProgressEventsInput, WorkflowProgressEventsResponse,
};
use a4n_core::event::channel::normalize_task_id;
use a4n_orchestrator::workflow::require_positive_workflow_run_id;

use super::super::super::{
    access::{
        verify_task_access, verify_task_handle_access, verify_task_project_access,
        verify_workflow_run_summary_access,
    },
    projections::{persisted_progress_event, task_progress_event},
};

pub async fn get_workflow_progress_events(
    state: &AppState,
    user_id: i64,
    input: WorkflowProgressEventsInput,
) -> Result<WorkflowProgressEventsResponse, String> {
    let task_id = match progress_events_target(&input)? {
        ProgressEventsTarget::Workflow(workflow_run_id) => {
            return workflow_progress_events_for_user(state, user_id, workflow_run_id, input.limit)
                .await
                .map(events_only);
        }
        ProgressEventsTarget::Task(task_id) => task_id,
    };
    if let Some(handle) = state.orchestrator().task_handle(&task_id) {
        verify_task_handle_access(state, user_id, &handle).await?;
        let events = handle
            .progress_events
            .read()
            .iter()
            .cloned()
            .rev()
            .take(input.limit)
            .map(task_progress_event)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        // 活体信号只在内存快照里，随每次轮询取最新值；历史事件不承载它。
        let stream_text = handle.progress.read().stream_text.clone();
        return Ok(WorkflowProgressEventsResponse {
            events,
            stream_text,
        });
    }
    if let Some(task) = state
        .orchestrator()
        .persisted_task_by_id(&task_id)
        .await
        .map_err(app_error_message)?
    {
        verify_task_access(state, user_id, &task_id).await?;
        return match task.workflow_run_id {
            Some(workflow_run_id) => {
                workflow_progress_events_for_user(state, user_id, workflow_run_id, input.limit)
                    .await
                    .map(events_only)
            }
            None => Ok(events_only(Vec::new())),
        };
    }
    if let Some(run) = state
        .orchestrator()
        .workflow_run_access_record_by_active_task_id(&task_id)
        .await
        .map_err(app_error_message)?
    {
        verify_task_project_access(state, user_id, run.project_id, run.root_project_id).await?;
        return state
            .orchestrator()
            .workflow_progress_events(run.id, input.limit)
            .await
            .map_err(app_error_message)
            .map(|events| events_only(events.into_iter().map(persisted_progress_event).collect()));
    }
    Err(format!("任务不存在: {task_id}"))
}

enum ProgressEventsTarget {
    Task(String),
    Workflow(i64),
}

fn progress_events_target(
    input: &WorkflowProgressEventsInput,
) -> Result<ProgressEventsTarget, String> {
    if !(1..=200).contains(&input.limit) {
        return Err("limit 必须在 1 到 200 之间".to_string());
    }
    let task_id = input
        .task_id
        .as_deref()
        .map(normalize_task_id)
        .transpose()
        .map_err(|error| error.to_string())?;
    match (task_id, input.workflow_run_id) {
        (Some(task_id), None) => Ok(ProgressEventsTarget::Task(task_id)),
        (None, Some(workflow_run_id)) => {
            require_positive_workflow_run_id(workflow_run_id)?;
            Ok(ProgressEventsTarget::Workflow(workflow_run_id))
        }
        _ => Err("必须且只能提供 task_id 或 workflow_run_id 之一".to_string()),
    }
}

/// 持久化路径没有内存快照，活体信号只能来自在跑的 task handle。
fn events_only(events: Vec<WorkflowProgressEvent>) -> WorkflowProgressEventsResponse {
    WorkflowProgressEventsResponse {
        events,
        stream_text: None,
    }
}

async fn workflow_progress_events_for_user(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
    limit: usize,
) -> Result<Vec<WorkflowProgressEvent>, String> {
    let run = state
        .orchestrator()
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)?;
    verify_workflow_run_summary_access(state, user_id, &run).await?;
    let events = state
        .orchestrator()
        .workflow_progress_events(workflow_run_id, limit)
        .await
        .map_err(app_error_message)?;
    Ok(events.into_iter().map(persisted_progress_event).collect())
}

#[cfg(test)]
mod tests {
    use super::{ProgressEventsTarget, progress_events_target};
    use a4n_api_contract::workflow::WorkflowProgressEventsInput;

    #[test]
    fn task_workflow_progress_events_target_requires_one_valid_target_and_bounded_limit() {
        let task = WorkflowProgressEventsInput {
            task_id: Some(" task-1 ".to_string()),
            workflow_run_id: None,
            limit: 1,
        };
        assert!(matches!(
            progress_events_target(&task).unwrap(),
            ProgressEventsTarget::Task(task_id) if task_id == "task-1"
        ));

        let workflow = WorkflowProgressEventsInput {
            task_id: None,
            workflow_run_id: Some(7),
            limit: 200,
        };
        assert!(matches!(
            progress_events_target(&workflow).unwrap(),
            ProgressEventsTarget::Workflow(7)
        ));

        let both = WorkflowProgressEventsInput {
            task_id: Some("task-1".to_string()),
            workflow_run_id: Some(7),
            limit: 20,
        };
        assert!(progress_events_target(&both).is_err());

        let invalid_limit = WorkflowProgressEventsInput {
            task_id: Some("task-1".to_string()),
            workflow_run_id: None,
            limit: 0,
        };
        assert!(progress_events_target(&invalid_limit).is_err());
    }
}
