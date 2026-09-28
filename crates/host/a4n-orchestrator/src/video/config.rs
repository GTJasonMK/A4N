pub use a4n_video::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
};

#[cfg(test)]
mod tests;

// 数据库视频配置到纯视频合同的唯一适配。

use a4n_db_models::models::ai::video_gen_config;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video::VideoConfigParams;
use a4n_video::{
    VideoDurationCapability, duration_capability_for_params,
    validate_video_duration_for_params_secs,
};

pub fn video_provider_config_from_model(
    cfg: &video_gen_config::Model,
) -> Result<VideoProviderConfig, String> {
    let pricing = serde_json::from_value(cfg.pricing_json.clone())
        .map_err(|error| format!("视频配置 #{} pricing_json 无效：{error}", cfg.id))?;
    Ok(VideoProviderConfig {
        id: cfg.id,
        name: cfg.name.clone(),
        backend: cfg.backend.clone(),
        base_url: cfg.base_url.clone(),
        api_key: cfg.api_key_encrypted.clone(),
        model: cfg.model.clone(),
        max_concurrent: cfg.max_concurrent,
        default_width: cfg.default_width,
        default_height: cfg.default_height,
        default_duration_secs: cfg.default_duration_secs,
        min_duration_secs: cfg.min_duration_secs,
        max_duration_secs: cfg.max_duration_secs,
        duration_mode: cfg.duration_mode.clone(),
        supported_durations_json: cfg.supported_durations_json.clone(),
        default_fps: cfg.default_fps,
        is_default: cfg.is_default,
        pricing,
    })
}

pub fn duration_capability_for_provider_config(
    cfg: &VideoProviderConfig,
) -> Result<VideoDurationCapability, String> {
    duration_capability_for_params(&video_config_params(cfg)?)
}

pub fn validate_video_duration_for_provider_config_secs(
    cfg: &VideoProviderConfig,
    duration_secs: i32,
) -> Result<(), String> {
    validate_video_duration_for_params_secs(&video_config_params(cfg)?, duration_secs)
}

fn video_config_params(cfg: &VideoProviderConfig) -> Result<VideoConfigParams, String> {
    Ok(VideoConfigParams {
        name: cfg.name.clone(),
        backend: cfg.backend.clone(),
        base_url: cfg
            .base_url
            .clone()
            .ok_or_else(|| "视频生成配置缺少 base_url".to_string())?,
        api_key_encrypted: cfg.api_key.clone(),
        model: cfg.model.clone(),
        max_concurrent: cfg.max_concurrent,
        default_width: cfg.default_width,
        default_height: cfg.default_height,
        default_duration_secs: cfg.default_duration_secs,
        min_duration_secs: cfg.min_duration_secs,
        max_duration_secs: cfg.max_duration_secs,
        duration_mode: cfg.duration_mode.clone(),
        supported_durations_json: cfg.supported_durations_json.clone(),
        default_fps: cfg.default_fps,
    })
}
