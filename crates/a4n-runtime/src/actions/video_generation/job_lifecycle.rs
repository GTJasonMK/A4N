#[cfg(test)]
use a4n_core::models::ai::generated_video;
#[cfg(test)]
use a4n_orchestrator::workflow::{
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_RUNNING,
};

#[cfg(test)]
use serde_json::{Value, json};

mod completion;
mod context;
mod polling;
mod workflow;

pub(super) use self::completion::save_completed_video_result;
pub(super) use self::context::{has_active_provider_job, normalized_provider_job_id};
pub(super) use self::polling::{
    poll_video_job, wait_for_canvas_graph_video_job_completion, wait_for_video_job_completion,
};

#[cfg(test)]
use self::completion::{cleanup_saved_video_files, saved_video_failure_result};
#[cfg(test)]
use self::context::{
    CanvasVideoWorkflowContext, active_provider_job_id, canvas_video_workflow_context,
    completed_provider_metadata, is_terminal_generated_video_status, provider_poll_config_id,
    required_video_chapter_id, should_capture_video_tail_frame,
};
#[cfg(test)]
use self::workflow::is_terminal_provider_workflow_checkpoint_status;

#[cfg(test)]
mod tests;
