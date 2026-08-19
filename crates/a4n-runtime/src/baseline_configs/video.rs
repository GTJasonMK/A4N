use super::env::{any_env_present, env_i32_or, env_string_or, optional_env_json};
use a4n_core::{config::env::required_non_empty_env_string, error::AppError};
use a4n_orchestrator::OrchestratorContext;

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
    orchestrator: &OrchestratorContext,
) -> Result<(), AppError> {
    if orchestrator.video_configs_exist().await? {
        return Ok(());
    }
    let Some(seed) = public_video_seed()? else {
        return Ok(());
    };
    let params = a4n_orchestrator::video::validate_video_config_params(seed)
        .map_err(AppError::InvalidArgument)?;
    orchestrator
        .insert_baseline_video_config_if_empty(params)
        .await?;
    Ok(())
}

fn public_video_seed() -> Result<Option<a4n_orchestrator::video::VideoConfigParams>, AppError> {
    if !any_env_present(PUBLIC_VIDEO_ENV_KEYS)? {
        return Ok(None);
    }
    Ok(Some(a4n_orchestrator::video::VideoConfigParams {
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
        duration_mode: env_string_or("A4N_PUBLIC_VIDEO_DURATION_MODE", "range")?,
        supported_durations_json: optional_env_json("A4N_PUBLIC_VIDEO_SUPPORTED_DURATIONS_JSON")?,
        default_fps: env_i32_or("A4N_PUBLIC_VIDEO_DEFAULT_FPS", 24)?,
    }))
}
