#[cfg(feature = "embedded-media-providers")]
use super::validation::validate_video_config_model;
use a4n_core::models::ai::video_gen_config;
use a4n_video::VideoProvider;
#[cfg(feature = "embedded-media-providers")]
use a4n_video_providers::pick_video_provider;

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
