use a4n_provider_contract::VideoProviderConfig;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use a4n_video::VideoJobState;
use tokio_util::sync::CancellationToken;

use crate::AppState;
use crate::actions::image_generation::run_queued_request_with_cancel_and_workflow;

pub(in crate::actions) async fn poll_provider_job(
    state: &AppState,
    cfg: &VideoProviderConfig,
    project_id: i64,
    root_project_id: Option<i64>,
    job_id: &str,
    workflow_run_id: i64,
    cancel_token: CancellationToken,
) -> Result<VideoJobState, String> {
    run_queued_request_with_cancel_and_workflow(
        state,
        QueueTaskKind::VideoPoll,
        TaskPriority::Low,
        Some(project_id),
        Some(workflow_run_id),
        cancel_token,
        || async {
            crate::sidecar_clients::poll_video(
                state.runtime_profile(),
                state.capabilities().video.clone(),
                cfg,
                job_id,
                crate::sidecar_clients::SidecarWorkflowScope {
                    project_id: Some(project_id),
                    root_project_id,
                    workflow_run_id: Some(workflow_run_id),
                },
            )
            .await
        },
    )
    .await
}
