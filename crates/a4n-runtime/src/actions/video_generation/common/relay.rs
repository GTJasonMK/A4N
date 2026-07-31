use std::future::Future;

use a4n_video::VideoGenerationResult;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::actions::video_generation::VideoGenerationProgress;
use crate::state::{CollaborationRuntimeFacade, RuntimeProfile};

use a4n_orchestrator::video::VIDEO_GENERATION_PROGRESS_TOTAL;

pub(in crate::actions) async fn video_sidecar_root_project_id(
    runtime_profile: RuntimeProfile,
    collaboration: &CollaborationRuntimeFacade<'_>,
    project_id: i64,
) -> Result<Option<i64>, String> {
    if !runtime_profile.uses_sidecars() {
        return Ok(None);
    }
    collaboration
        .collaboration_root_project(project_id)
        .await
        .map(|root| Some(root.id))
        .map_err(crate::actions::app_error_message)
}

pub(in crate::actions::video_generation) async fn relay_video_sidecar_progress<F>(
    progress: &mut VideoGenerationProgress<'_>,
    mut progress_rx: UnboundedReceiver<String>,
    future: F,
) -> Result<VideoGenerationResult, String>
where
    F: Future<Output = Result<VideoGenerationResult, String>>,
{
    tokio::pin!(future);
    let mut progress_open = true;
    loop {
        tokio::select! {
            result = &mut future => {
                while let Ok(message) = progress_rx.try_recv() {
                    progress
                        .progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, &message)
                        .map_err(|error| error.to_string())?;
                }
                return result;
            }
            message = progress_rx.recv(), if progress_open => {
                match message {
                    Some(message) => {
                        progress
                            .progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, &message)
                            .map_err(|error| error.to_string())?;
                    }
                    None => progress_open = false,
                }
            }
        }
    }
}
