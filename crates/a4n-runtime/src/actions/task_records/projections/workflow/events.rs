use a4n_core::event::channel::TaskProgressEvent;
use a4n_queue::priority::TaskProgress as QueueTaskProgress;

use super::super::super::TaskProgressEventSummary;

pub(in crate::actions::task_records) fn task_progress_event_summary(
    task_id: &str,
    event: TaskProgressEvent,
) -> TaskProgressEventSummary {
    TaskProgressEventSummary {
        id: event.id,
        task_id: Some(task_id.to_string()),
        current: event.current,
        total: event.total,
        message: event.message,
        percent: if event.total == 0 {
            0.0
        } else {
            event.current as f32 / event.total as f32 * 100.0
        },
        meta: event.meta,
        timestamp: event.timestamp,
    }
}

pub(in crate::actions::task_records::projections) fn queue_progress_event_summary(
    task_id: &str,
    progress: &QueueTaskProgress,
    timestamp: chrono::NaiveDateTime,
) -> TaskProgressEventSummary {
    TaskProgressEventSummary {
        id: format!("{task_id}-snapshot"),
        task_id: Some(task_id.to_string()),
        current: progress.current,
        total: progress.total,
        message: progress.message.clone(),
        percent: progress.percent() * 100.0,
        meta: progress.meta.clone(),
        timestamp: progress
            .updated_at
            .clone()
            .unwrap_or_else(|| timestamp.to_string()),
    }
}
