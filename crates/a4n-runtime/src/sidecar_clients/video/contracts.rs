use a4n_provider_contract::VideoProviderConfig;
use a4n_video::{VideoConfigParams, VideoGenerationRequest, VideoGenerationResult};
use base64::Engine as _;

pub(in crate::sidecar_clients::video) use a4n_video_sidecar_contract::{
    VideoDeleteOutput, VideoGenerationOutput, VideoJobOutput, VideoPollPayload,
    VideoProviderPayload, VideoStatusOutput,
};

pub(in crate::sidecar_clients::video) fn video_provider_payload(
    config: &VideoProviderConfig,
    request: VideoGenerationRequest,
    provider_idempotency_key_sha256: Option<String>,
    project_id: Option<i64>,
) -> Result<VideoProviderPayload, String> {
    let params = video_config_params(config)?;
    let api_key = params
        .api_key_encrypted
        .ok_or_else(|| "视频生成配置缺少 api_key_encrypted".to_string())?;
    Ok(VideoProviderPayload {
        backend: params.backend,
        base_url: params.base_url,
        api_key,
        provider_idempotency_key_sha256,
        project_id,
        request,
    })
}

pub(in crate::sidecar_clients::video) fn video_config_params(
    config: &VideoProviderConfig,
) -> Result<VideoConfigParams, String> {
    a4n_video::validate_video_config_params(VideoConfigParams {
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config
            .base_url
            .clone()
            .ok_or_else(|| "视频生成配置缺少 base_url".to_string())?,
        api_key_encrypted: config.api_key.clone(),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        default_width: config.default_width,
        default_height: config.default_height,
        default_duration_secs: config.default_duration_secs,
        min_duration_secs: config.min_duration_secs,
        max_duration_secs: config.max_duration_secs,
        duration_mode: config.duration_mode.clone(),
        supported_durations_json: config.supported_durations_json.clone(),
        default_fps: config.default_fps,
    })
}

pub(in crate::sidecar_clients::video) fn video_generation_result(
    output: VideoGenerationOutput,
) -> Result<VideoGenerationResult, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(output.bytes_base64)
        .map_err(|err| format!("video sidecar 返回的 bytes_base64 无效: {err}"))?;
    Ok(VideoGenerationResult {
        bytes,
        format: output.format,
        provider_job_id: output.provider_job_id,
        metadata: output.metadata,
    })
}
