use a4n_core::models::ai::video_gen_config;
use a4n_queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_video::VideoJobState;
use tokio_util::sync::CancellationToken;

use crate::AppState;
use crate::actions::image_generation::run_queued_request_with_cancel_and_workflow;

pub(super) async fn poll_provider_job(
    state: &AppState,
    cfg: &video_gen_config::Model,
    project_id: i64,
    root_project_id: Option<i64>,
    job_id: &str,
    priority: TaskPriority,
    workflow_run_id: Option<i64>,
    cancel_token: CancellationToken,
) -> Result<VideoJobState, String> {
    run_queued_request_with_cancel_and_workflow(
        state,
        QueueTaskType::VideoPoll,
        priority,
        Some(project_id),
        workflow_run_id,
        cancel_token,
        || async {
            crate::sidecar_clients::poll_video(
                state.runtime_profile,
                state.capabilities(),
                cfg,
                job_id,
                Some(project_id),
                root_project_id,
                workflow_run_id,
            )
            .await
        },
    )
    .await
}
