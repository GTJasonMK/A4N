use a4n_api_contract::workflow::WorkflowProgressEvent;
use a4n_task_runtime::channel::TaskProgressEvent;

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
