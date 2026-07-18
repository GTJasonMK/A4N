use a4n_core::models::ai::{generated_video, video_gen_config};

use super::{AppState, VideoGenerationProgress, wait_for_video_job_completion_inner};

pub(in crate::actions::video_generation) async fn wait_for_video_job_completion(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    video: generated_video::Model,
    cfg: &video_gen_config::Model,
    job_id: &str,
) -> Result<generated_video::Model, String> {
    wait_for_video_job_completion_inner(state, progress, video, cfg, job_id, true).await
}

pub(in crate::actions::video_generation) async fn wait_for_canvas_graph_video_job_completion(
    state: &AppState,
    progress: &mut VideoGenerationProgress<'_>,
    video: generated_video::Model,
    cfg: &video_gen_config::Model,
    job_id: &str,
) -> Result<generated_video::Model, String> {
    wait_for_video_job_completion_inner(state, progress, video, cfg, job_id, false).await
}
