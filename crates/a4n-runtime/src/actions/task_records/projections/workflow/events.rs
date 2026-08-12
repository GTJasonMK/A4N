use a4n_api_contract::workflow::WorkflowProgressEvent;
use a4n_core::event::channel::TaskProgressEvent;
use a4n_db::workflow::TaskProgressEventSummary;
use a4n_queue::priority::TaskProgress as QueueTaskProgress;

pub(in crate::actions::task_records) fn task_progress_event(
    event: TaskProgressEvent,
) -> WorkflowProgressEvent {
    WorkflowProgressEvent {
        id: event.id,
        current: event.current,
        total: event.total,
        message: event.message,
        meta: event.meta,
        timestamp: event.timestamp,
    }
}

pub(in crate::actions::task_records::projections) fn queue_progress_event_summary(
    task_id: &str,
    progress: &QueueTaskProgress,
    timestamp: chrono::NaiveDateTime,
) -> WorkflowProgressEvent {
    WorkflowProgressEvent {
        id: format!("{task_id}-snapshot"),
        current: progress.current,
        total: progress.total,
        message: progress.message.clone(),
        meta: progress.meta.clone(),
        timestamp: progress
            .updated_at
            .clone()
            .unwrap_or_else(|| timestamp.to_string()),
    }
}

pub(in crate::actions::task_records) fn persisted_progress_event(
    event: TaskProgressEventSummary,
) -> WorkflowProgressEvent {
    WorkflowProgressEvent {
        id: event.id,
        current: event.current,
        total: event.total,
        message: event.message,
        meta: event.meta,
        timestamp: event.timestamp,
    }
}
