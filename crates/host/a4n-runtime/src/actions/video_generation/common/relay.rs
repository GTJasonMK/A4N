use std::future::Future;

use a4n_error::AppError;
use a4n_video::VideoGenerationResult;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::state::RuntimeProfile;
use a4n_video_host_contract::VideoGenerationProgressPort;

use a4n_domain::video_workflow::VIDEO_GENERATION_PROGRESS_TOTAL;

pub(in crate::actions) async fn video_sidecar_root_project_id(
    runtime_profile: RuntimeProfile,
    collaboration: &dyn a4n_api_contract::collaboration::ProjectAccessApplicationPort,
    project_id: i64,
) -> Result<Option<i64>, String> {
    if !runtime_profile.uses_sidecars() {
        return Ok(None);
    }
    collaboration
        .collaboration_root(project_id)
        .await
        .map(|root| Some(root.id))
        .map_err(crate::actions::app_error_message)
}

pub(in crate::actions::video_generation) async fn relay_video_sidecar_progress<F>(
    progress: &mut dyn VideoGenerationProgressPort,
    mut progress_rx: UnboundedReceiver<String>,
    future: F,
) -> Result<VideoGenerationResult, AppError>
where
    F: Future<Output = Result<VideoGenerationResult, AppError>>,
{
    tokio::pin!(future);
    let mut progress_open = true;
    loop {
        tokio::select! {
            result = &mut future => {
                while let Ok(message) = progress_rx.try_recv() {
                    progress
                        .progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, &message)
                        ?;
                }
                return result;
            }
            message = progress_rx.recv(), if progress_open => {
                match message {
                    Some(message) => {
                        progress
                            .progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, &message)
                            ?;
                    }
                    None => progress_open = false,
                }
            }
        }
    }
}
