use super::env::{any_env_present, env_i32_or, env_string_or, optional_env_json};
use a4n_api_contract::config::{
    ConfigMutationApplicationPort, ConfigReadApplicationPort, VideoConfigInput, VideoDurationMode,
};
use a4n_config::required_non_empty_env_string;
use a4n_error::AppError;

const PUBLIC_VIDEO_ENV_KEYS: &[&str] = &[
    "A4N_PUBLIC_VIDEO_NAME",
    "A4N_PUBLIC_VIDEO_BACKEND",
    "A4N_PUBLIC_VIDEO_BASE_URL",
    "A4N_PUBLIC_VIDEO_API_KEY",
    "A4N_PUBLIC_VIDEO_MODEL",
    "A4N_PUBLIC_VIDEO_MAX_CONCURRENT",
    "A4N_PUBLIC_VIDEO_DEFAULT_WIDTH",
    "A4N_PUBLIC_VIDEO_DEFAULT_HEIGHT",
    "A4N_PUBLIC_VIDEO_DEFAULT_DURATION_SECS",
    "A4N_PUBLIC_VIDEO_MIN_DURATION_SECS",
    "A4N_PUBLIC_VIDEO_MAX_DURATION_SECS",
    "A4N_PUBLIC_VIDEO_DURATION_MODE",
    "A4N_PUBLIC_VIDEO_SUPPORTED_DURATIONS_JSON",
    "A4N_PUBLIC_VIDEO_DEFAULT_FPS",
];

pub(super) async fn ensure_public_video_config(
    config_read_application: &dyn ConfigReadApplicationPort,
    config_mutation_application: &dyn ConfigMutationApplicationPort,
) -> Result<(), AppError> {
    if config_read_application.video_configs_exist().await? {
        return Ok(());
    }
    let Some(seed) = public_video_seed()? else {
        return Ok(());
    };
    config_mutation_application
        .insert_baseline_video_if_empty(seed)
        .await?;
    Ok(())
}

fn public_video_seed() -> Result<Option<VideoConfigInput>, AppError> {
    if !any_env_present(PUBLIC_VIDEO_ENV_KEYS)? {
        return Ok(None);
    }
    let duration_mode = env_string_or("A4N_PUBLIC_VIDEO_DURATION_MODE", "range")?
        .parse::<VideoDurationMode>()
        .map_err(|error| AppError::ConfigError(error.to_string()))?;
    Ok(Some(VideoConfigInput {
        name: env_string_or("A4N_PUBLIC_VIDEO_NAME", "公共基础视频")?,
        backend: env_string_or("A4N_PUBLIC_VIDEO_BACKEND", "openai-compatible-video")?,
        base_url: required_non_empty_env_string("A4N_PUBLIC_VIDEO_BASE_URL")?,
        api_key_encrypted: Some(required_non_empty_env_string("A4N_PUBLIC_VIDEO_API_KEY")?),
        model: required_non_empty_env_string("A4N_PUBLIC_VIDEO_MODEL")?,
        max_concurrent: env_i32_or("A4N_PUBLIC_VIDEO_MAX_CONCURRENT", 1)?,
        default_width: env_i32_or("A4N_PUBLIC_VIDEO_DEFAULT_WIDTH", 1280)?,
        default_height: env_i32_or("A4N_PUBLIC_VIDEO_DEFAULT_HEIGHT", 720)?,
        default_duration_secs: env_i32_or("A4N_PUBLIC_VIDEO_DEFAULT_DURATION_SECS", 5)?,
        min_duration_secs: env_i32_or("A4N_PUBLIC_VIDEO_MIN_DURATION_SECS", 1)?,
        max_duration_secs: env_i32_or("A4N_PUBLIC_VIDEO_MAX_DURATION_SECS", 60)?,
        duration_mode,
        supported_durations_json: optional_env_json("A4N_PUBLIC_VIDEO_SUPPORTED_DURATIONS_JSON")?,
        default_fps: env_i32_or("A4N_PUBLIC_VIDEO_DEFAULT_FPS", 24)?,
        pricing: Default::default(),
    }))
}
