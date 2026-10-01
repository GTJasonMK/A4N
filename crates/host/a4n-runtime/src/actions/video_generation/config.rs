use a4n_config::optional_non_empty_env_string;
use a4n_error::AppError;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video::ProviderImageInputTransport;

use crate::{AppState, actions::video_generation::media::remote_temp_upload_config};

use super::provider_host::host_error;

pub(super) async fn resolve_provider_config(
    state: &AppState,
    config_id: Option<i64>,
) -> Result<VideoProviderConfig, AppError> {
    crate::actions::video_generation::resolve_video_config(
        state.video_provider_config(),
        config_id,
        state.provider_runtime().allows_host_provider_config(),
    )
    .await
    .map_err(host_error)
}

pub(super) fn resolve_image_transport(
    backend: &str,
) -> Result<ProviderImageInputTransport, AppError> {
    let override_value = optional_non_empty_env_string("A4N_VIDEO_IMAGE_INPUT_TRANSPORT")?;
    if let Some(transport) =
        a4n_video::provider_image_input_transport_for_backend(backend, override_value.as_deref())
            .map_err(host_error)?
    {
        return Ok(transport);
    }

    let remote_url = optional_non_empty_env_string("A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL")?;
    let remote_token = optional_non_empty_env_string("A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN")?;
    match (remote_url, remote_token) {
        (Some(_), Some(_)) => {
            remote_temp_upload_config().map_err(host_error)?;
            return Ok(ProviderImageInputTransport::RemoteTempUpload);
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(host_error(
                "auto 传输已检测到不完整的远端临时媒体配置；A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL 和 A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN 必须同时配置".to_string(),
            ));
        }
        (None, None) => {}
    }

    if optional_non_empty_env_string("A4N_PUBLIC_APP_URL")?.is_some() {
        return Ok(ProviderImageInputTransport::PublicUrl);
    }
    Ok(ProviderImageInputTransport::InlineDataUrl)
}
