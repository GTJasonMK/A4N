use tokio_util::sync::CancellationToken;

use crate::AppState;
use crate::actions::video_generation::config_state::resolve_video_config;
use crate::actions::video_generation::media::prompt_id_from_video;
use crate::actions::video_generation::provider_host::RuntimeVideoProviderHostPort;
use crate::actions::video_generation::{GeneratedVideoSummary, video_to_summary};
use a4n_orchestrator::video::provider_poll_config_id;

pub(in crate::actions::video_generation) async fn poll_video_job(
    state: &AppState,
    video: a4n_core::models::ai::generated_video::Model,
) -> Result<GeneratedVideoSummary, String> {
    if !a4n_orchestrator::video::has_active_provider_job(
        video.provider_job_id.as_deref(),
        &video.status,
    ) {
        return Ok(video_to_summary(&video));
    }
    let cfg_id = provider_poll_config_id(&video)?;
    let cfg = resolve_video_config(state.orchestrator(), cfg_id).await?;
    let prompt_id = prompt_id_from_video(&video);
    let host_port = RuntimeVideoProviderHostPort::new(state);
    let updated = state
        .orchestrator()
        .poll_video_provider_once_validated(
            video,
            &cfg,
            prompt_id,
            CancellationToken::new(),
            &host_port,
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(video_to_summary(&updated))
}
