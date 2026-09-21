use super::{GeneratedVideoSummary, provider_host::RuntimeVideoProviderHostPort};
use crate::AppState;
use a4n_error::AppError;
use a4n_video_host_contract::VideoProviderPollTarget;

pub async fn poll_video_job(
    state: &AppState,
    user_id: i64,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    let host_port = RuntimeVideoProviderHostPort::new(state);
    let target = state
        .video_provider_execution()
        .poll_target(user_id, video_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    let config = match target {
        VideoProviderPollTarget::Inactive => None,
        VideoProviderPollTarget::Active { config_id } => Some(
            super::resolve_video_config(state.video_provider_config(), config_id)
                .await
                .map_err(|operation| AppError::InvalidState {
                    current: "video_provider_config".to_string(),
                    operation,
                })
                .map_err(crate::actions::app_error_message)?,
        ),
    };
    state
        .video_provider_execution()
        .poll_provider_job(user_id, video_id, config.as_ref(), &host_port)
        .await
        .map_err(crate::actions::app_error_message)
}
