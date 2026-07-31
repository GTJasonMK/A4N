pub(in crate::workflow) const MAX_PERSISTED_PROGRESS_EVENTS: usize = 200;

pub const WORKFLOW_STATUS_RUNNING: &str = "running";
pub const WORKFLOW_STATUS_INTERRUPTED: &str = "interrupted";
pub const WORKFLOW_STATUS_FAILED: &str = "failed";
pub const WORKFLOW_STATUS_CANCELLED: &str = "cancelled";
pub const WORKFLOW_STATUS_COMPLETED: &str = "completed";
pub const WORKFLOW_STATUS_SUPERSEDED: &str = "superseded";

pub(in crate::workflow) const WORKFLOW_STATUSES: &[&str] = &[
    WORKFLOW_STATUS_RUNNING,
    WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_CANCELLED,
    WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_SUPERSEDED,
];

pub(in crate::workflow) fn is_terminal_workflow_status(status: &str) -> bool {
    matches!(
        status,
        WORKFLOW_STATUS_COMPLETED
            | WORKFLOW_STATUS_FAILED
            | WORKFLOW_STATUS_CANCELLED
            | WORKFLOW_STATUS_SUPERSEDED
    )
}
