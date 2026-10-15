use crate::{AppState, actions::app_error_message};
use a4n_api_contract::workflow::{
    WorkflowProgressEvent, WorkflowProgressEventsInput, WorkflowProgressEventsResponse,
};
use a4n_domain::workflow::require_positive_workflow_run_id;
use a4n_task_runtime::channel::{TaskProgressEvent, normalize_task_id};
use std::collections::VecDeque;

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
    input: WorkflowProgressEventsInput,
) -> Result<WorkflowProgressEventsResponse, String> {
    let after_event_id = input.after_event_id.as_deref();
    let task_id = match progress_events_target(&input)? {
        ProgressEventsTarget::Workflow(workflow_run_id) => {
            let cursor = parse_workflow_event_cursor(after_event_id)?;
            return workflow_progress_events_for_user(
                state,
                user_id,
                workflow_run_id,
                input.limit,
                cursor,
            )
            .await
            .map(events_only);
        }
        ProgressEventsTarget::Task(task_id) => task_id,
    };
    if let Some(handle) = state.task_manager().get(&task_id) {
        verify_task_handle_access(state, user_id, &handle).await?;
        let events =
            task_progress_events_since(&handle.progress_events.read(), input.limit, after_event_id);
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
                let cursor = after_event_id.and_then(parse_positive_workflow_event_cursor);
                workflow_progress_events_for_user(
                    state,
                    user_id,
                    workflow_run_id,
                    input.limit,
                    cursor,
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
        let cursor = after_event_id.and_then(parse_positive_workflow_event_cursor);
        return state
            .workflow_task_lookup_application()
            .workflow_progress_events(run.workflow_run_id, input.limit, cursor)
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

fn task_progress_events_since(
    events: &VecDeque<TaskProgressEvent>,
    limit: usize,
    after_event_id: Option<&str>,
) -> Vec<WorkflowProgressEvent> {
    let start = after_event_id
        .and_then(|cursor| {
            events
                .iter()
                .rposition(|event| event.id == cursor)
                .map(|index| index + 1)
        })
        .unwrap_or_else(|| events.len().saturating_sub(limit));
    events
        .iter()
        .skip(start)
        .take(limit)
        .cloned()
        .map(task_progress_event)
        .collect()
}

fn parse_workflow_event_cursor(value: Option<&str>) -> Result<Option<i64>, String> {
    value
        .map(|value| {
            parse_positive_workflow_event_cursor(value)
                .ok_or_else(|| "workflow after_event_id 必须是正整数".to_string())
        })
        .transpose()
}

fn parse_positive_workflow_event_cursor(value: &str) -> Option<i64> {
    value.parse::<i64>().ok().filter(|event_id| *event_id > 0)
}

fn progress_events_target(
    input: &WorkflowProgressEventsInput,
) -> Result<ProgressEventsTarget, String> {
    if !(1..=200).contains(&input.limit) {
        return Err("limit 必须在 1 到 200 之间".to_string());
    }
    if input
        .after_event_id
        .as_ref()
        .is_some_and(|event_id| event_id.is_empty() || event_id.len() > 256)
    {
        return Err("after_event_id 长度必须在 1 到 256 字节之间".to_string());
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
        ProgressEventsTarget, parse_workflow_event_cursor, progress_events_target,
        task_progress_events_since,
    };
    use a4n_api_contract::workflow::WorkflowProgressEventsInput;
    use a4n_task_runtime::channel::TaskProgressEvent;
    use std::collections::VecDeque;

    #[test]
    fn task_workflow_progress_events_target_requires_one_valid_target_and_bounded_limit() {
        let task = WorkflowProgressEventsInput {
            task_id: Some(" task-1 ".to_string()),
            workflow_run_id: None,
            after_event_id: None,
            limit: 1,
        };
        assert!(matches!(
            progress_events_target(&task).unwrap(),
            ProgressEventsTarget::Task(task_id) if task_id == "task-1"
        ));

        let workflow = WorkflowProgressEventsInput {
            task_id: None,
            workflow_run_id: Some(7),
            after_event_id: None,
            limit: 200,
        };
        assert!(matches!(
            progress_events_target(&workflow).unwrap(),
            ProgressEventsTarget::Workflow(7)
        ));

        let both = WorkflowProgressEventsInput {
            task_id: Some("task-1".to_string()),
            workflow_run_id: Some(7),
            after_event_id: None,
            limit: 20,
        };
        assert!(progress_events_target(&both).is_err());

        let invalid_limit = WorkflowProgressEventsInput {
            task_id: Some("task-1".to_string()),
            workflow_run_id: None,
            after_event_id: None,
            limit: 0,
        };
        assert!(progress_events_target(&invalid_limit).is_err());

        let invalid_cursor = WorkflowProgressEventsInput {
            task_id: Some("task-1".to_string()),
            workflow_run_id: None,
            after_event_id: Some("x".repeat(257)),
            limit: 20,
        };
        assert!(progress_events_target(&invalid_cursor).is_err());
    }

    #[test]
    fn task_progress_event_cursor_returns_only_unseen_events_in_order() {
        let events = VecDeque::from([event("event-1"), event("event-2"), event("event-3")]);

        let incremental = task_progress_events_since(&events, 200, Some("event-2"));
        assert_eq!(
            incremental
                .iter()
                .map(|event| event.id.as_str())
                .collect::<Vec<_>>(),
            ["event-3"]
        );

        let stale_cursor = task_progress_events_since(&events, 2, Some("stale"));
        assert_eq!(
            stale_cursor
                .iter()
                .map(|event| event.id.as_str())
                .collect::<Vec<_>>(),
            ["event-2", "event-3"]
        );
    }

    #[test]
    fn workflow_event_cursor_requires_a_positive_database_event_id() {
        assert_eq!(parse_workflow_event_cursor(None).unwrap(), None);
        assert_eq!(parse_workflow_event_cursor(Some("42")).unwrap(), Some(42));
        assert!(parse_workflow_event_cursor(Some("task-1-20")).is_err());
        assert!(parse_workflow_event_cursor(Some("0")).is_err());
    }

    fn event(id: &str) -> TaskProgressEvent {
        TaskProgressEvent {
            id: id.to_string(),
            current: 1,
            total: 3,
            message: id.to_string(),
            meta: None,
            timestamp: "2026-10-15T00:00:00Z".to_string(),
        }
    }
}
