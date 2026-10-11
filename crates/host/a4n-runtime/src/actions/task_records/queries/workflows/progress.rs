use crate::{AppState, actions::app_error_message};
use a4n_api_contract::workflow::{
    GetWorkflowProgressEventsInput, WorkflowProgressEvent, WorkflowProgressEventsResponse,
};
use a4n_domain::workflow::require_positive_workflow_run_id;
use a4n_task_runtime::channel::normalize_task_id;

use super::super::super::{
    access::{
        verify_task_access, verify_task_handle_access, verify_task_project_access,
        verify_workflow_run_summary_access,
    },
    projections::task_progress_event,
};

pub async fn get_workflow_progress_events(
    state: &AppState,
    user_id: i64,
    input: GetWorkflowProgressEventsInput,
) -> Result<WorkflowProgressEventsResponse, String> {
    let task_id = match progress_events_target(&input)? {
        ProgressEventsTarget::Workflow(workflow_run_id) => {
            let after_event_id = parse_persisted_event_id(input.after_event_id.as_deref())?;
            return workflow_progress_events_for_user(
                state,
                user_id,
                workflow_run_id,
                input.limit,
                after_event_id,
            )
            .await
            .map(events_only);
        }
        ProgressEventsTarget::Task(task_id) => task_id,
    };
    if let Some(handle) = state.task_manager().get(&task_id) {
        verify_task_handle_access(state, user_id, &handle).await?;
        let events = handle
            .progress_events_after(input.after_event_id.as_deref(), input.limit)
            .into_iter()
            .map(task_progress_event)
            .collect();
        // 活体信号只在内存快照里，随每次轮询取最新值；历史事件不承载它。
        let stream_text = handle.progress.read().stream_text.clone();
        return Ok(WorkflowProgressEventsResponse {
            events,
            stream_text,
        });
    }
    if let Some(task) = state
        .workflow_task_lookup_application()
        .task_by_id(&task_id)
        .await
        .map_err(app_error_message)?
    {
        verify_task_access(state, user_id, &task_id).await?;
        return match task.workflow_run_id {
            Some(workflow_run_id) => {
                let after_event_id = task_fallback_event_cursor(input.after_event_id.as_deref())?;
                workflow_progress_events_for_user(
                    state,
                    user_id,
                    workflow_run_id,
                    input.limit,
                    after_event_id,
                )
                .await
                .map(events_only)
            }
            None => Ok(events_only(Vec::new())),
        };
    }
    if let Some(run) = state
        .workflow_task_lookup_application()
        .workflow_access_by_active_task_id(&task_id)
        .await
        .map_err(app_error_message)?
    {
        verify_task_project_access(state, user_id, run.project_id, run.root_project_id).await?;
        return state
            .workflow_task_lookup_application()
            .workflow_progress_events(
                run.workflow_run_id,
                input.limit,
                task_fallback_event_cursor(input.after_event_id.as_deref())?,
            )
            .await
            .map_err(app_error_message)
            .map(events_only);
    }
    Err(format!("任务不存在: {task_id}"))
}

enum ProgressEventsTarget {
    Task(String),
    Workflow(i64),
}

fn progress_events_target(
    input: &GetWorkflowProgressEventsInput,
) -> Result<ProgressEventsTarget, String> {
    if !(1..=200).contains(&input.limit) {
        return Err("limit 必须在 1 到 200 之间".to_string());
    }
    if input
        .after_event_id
        .as_ref()
        .is_some_and(|event_id| event_id.is_empty() || event_id.len() > 256)
    {
        return Err("after_event_id 长度必须在 1 到 256 之间".to_string());
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

fn parse_persisted_event_id(event_id: Option<&str>) -> Result<Option<i64>, String> {
    event_id
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|_| "工作流进度事件 ID 必须是正整数".to_string())
                .and_then(|id| {
                    (id > 0)
                        .then_some(id)
                        .ok_or_else(|| "工作流进度事件 ID 必须是正整数".to_string())
                })
        })
        .transpose()
}

fn task_fallback_event_cursor(event_id: Option<&str>) -> Result<Option<i64>, String> {
    let Some(event_id) = event_id else {
        return Ok(None);
    };
    // Once a task handle is gone, its opaque event IDs cannot address persisted run events.
    match event_id.parse::<i64>() {
        Ok(id) if id > 0 => Ok(Some(id)),
        Ok(_) => Err("工作流进度事件 ID 必须是正整数".to_string()),
        Err(_) => Ok(None),
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
    after_event_id: Option<i64>,
) -> Result<Vec<WorkflowProgressEvent>, String> {
    let run = state
        .workflow_task_lookup_application()
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)?;
    verify_workflow_run_summary_access(state, user_id, &run).await?;
    let events = state
        .workflow_task_lookup_application()
        .workflow_progress_events(workflow_run_id, limit, after_event_id)
        .await
        .map_err(app_error_message)?;
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::{
        ProgressEventsTarget, parse_persisted_event_id, progress_events_target,
        task_fallback_event_cursor,
    };
    use a4n_api_contract::workflow::GetWorkflowProgressEventsInput;

    #[test]
    fn task_workflow_progress_events_target_requires_one_valid_target_and_bounded_limit() {
        let task = GetWorkflowProgressEventsInput {
            task_id: Some(" task-1 ".to_string()),
            workflow_run_id: None,
            after_event_id: Some("task-1-2026-10-11T00:00:00Z".to_string()),
            limit: 1,
        };
        assert!(matches!(
            progress_events_target(&task).unwrap(),
            ProgressEventsTarget::Task(task_id) if task_id == "task-1"
        ));

        let workflow = GetWorkflowProgressEventsInput {
            task_id: None,
            workflow_run_id: Some(7),
            after_event_id: Some("11".to_string()),
            limit: 200,
        };
        assert!(matches!(
            progress_events_target(&workflow).unwrap(),
            ProgressEventsTarget::Workflow(7)
        ));

        let both = GetWorkflowProgressEventsInput {
            task_id: Some("task-1".to_string()),
            workflow_run_id: Some(7),
            after_event_id: None,
            limit: 20,
        };
        assert!(progress_events_target(&both).is_err());

        let invalid_limit = GetWorkflowProgressEventsInput {
            task_id: Some("task-1".to_string()),
            workflow_run_id: None,
            after_event_id: None,
            limit: 0,
        };
        assert!(progress_events_target(&invalid_limit).is_err());
    }

    #[test]
    fn persisted_event_cursors_require_positive_numeric_ids() {
        assert_eq!(parse_persisted_event_id(None).unwrap(), None);
        assert_eq!(parse_persisted_event_id(Some("17")).unwrap(), Some(17));
        assert!(parse_persisted_event_id(Some("task-1-17")).is_err());
        assert!(parse_persisted_event_id(Some("0")).is_err());

        assert_eq!(task_fallback_event_cursor(Some("task-1-17")).unwrap(), None);
        assert_eq!(task_fallback_event_cursor(Some("17")).unwrap(), Some(17));
    }
}
