use super::contracts::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, VIDEO_DURATION_MODE_FIXED, VideoConfigParams,
};
#[cfg(feature = "embedded-media-providers")]
use super::validation::validate_video_config_model;
use a4n_core::models::ai::video_gen_config;
use a4n_video::VideoProvider;
#[cfg(feature = "embedded-media-providers")]
use a4n_video_providers::pick_video_provider;

pub(super) fn validate_backend_duration_constraints(
    params: &VideoConfigParams,
) -> Result<(), String> {
    if params.backend != CLIPROXY_OPENAI_VIDEO_BACKEND {
        return Ok(());
    }
    if params.min_duration_secs < 1 || params.max_duration_secs > 15 {
        return Err("CLIProxy / Grok 视频配置支持时长范围必须在 1-15 秒之间".to_string());
    }
    if params.default_duration_secs > 15 {
        return Err("CLIProxy / Grok 视频默认时长不能超过 15 秒".to_string());
    }
    if params.duration_mode == VIDEO_DURATION_MODE_FIXED {
        let supported = params
            .supported_durations_json
            .as_ref()
            .and_then(|value| value.as_array())
            .ok_or_else(|| "CLIProxy / Grok 视频 fixed 模式必须填写固定时长档位".to_string())?;
        if supported
            .iter()
            .any(|value| value.as_i64().unwrap_or(0) > 15)
        {
            return Err("CLIProxy / Grok 视频固定时长档位不能超过 15 秒".to_string());
        }
    }
    Ok(())
}

pub fn pick_provider_for_config(
    cfg: &video_gen_config::Model,
) -> Result<Box<dyn VideoProvider>, String> {
    #[cfg(feature = "embedded-media-providers")]
    {
        let params = validate_video_config_model(cfg)?;
        let key = params
            .api_key_encrypted
            .as_deref()
            .ok_or_else(|| "视频生成配置缺少 api_key_encrypted".to_string())?;
        pick_video_provider(&params.backend, &params.base_url, key).map_err(|err| err.to_string())
    }
    #[cfg(not(feature = "embedded-media-providers"))]
    {
        let _ = cfg;
        Err(
            "视频 provider 未启用：请使用 video sidecar，或用 embedded-media-providers feature 构建"
                .to_string(),
        )
    }
}
