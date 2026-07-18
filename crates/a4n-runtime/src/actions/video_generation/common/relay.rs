use std::future::Future;

use a4n_video::VideoGenerationResult;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::AppState;
use crate::actions::video_generation::VideoGenerationProgress;

use super::constants::VIDEO_GENERATION_PROGRESS_TOTAL;

pub(in crate::actions::video_generation) async fn video_sidecar_root_project_id(
    state: &AppState,
    project_id: i64,
) -> Result<Option<i64>, String> {
    crate::actions::sidecar_root_project_id(state, project_id).await
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
                    progress.progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, &message)?;
                }
                return result;
            }
            message = progress_rx.recv(), if progress_open => {
                match message {
                    Some(message) => {
                        progress.progress(3, VIDEO_GENERATION_PROGRESS_TOTAL, &message)?;
                    }
                    None => progress_open = false,
                }
            }
        }
    }
}
