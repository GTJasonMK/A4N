pub const WORKFLOW_STATUS_RUNNING: &str = "running";
pub const WORKFLOW_STATUS_INTERRUPTED: &str = "interrupted";
pub const WORKFLOW_STATUS_FAILED: &str = "failed";
pub const WORKFLOW_STATUS_CANCELLED: &str = "cancelled";
pub const WORKFLOW_STATUS_COMPLETED: &str = "completed";
pub const WORKFLOW_STATUS_SUPERSEDED: &str = "superseded";
pub const WORKFLOW_MODE_DERIVED_REPLAY: &str = "derived_replay";

pub fn is_video_status_final_for_cancel_interrupt(status: &str) -> bool {
    matches!(
        status,
        WORKFLOW_STATUS_COMPLETED
            | WORKFLOW_STATUS_FAILED
            | WORKFLOW_STATUS_CANCELLED
            | WORKFLOW_STATUS_INTERRUPTED
    )
}
