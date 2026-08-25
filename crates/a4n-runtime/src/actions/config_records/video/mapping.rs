use super::super::shared::has_configured_api_key;
use a4n_api_contract::config::VideoConfigSummary;
use a4n_core::models::ai::video_gen_config;
use std::str::FromStr;

pub(super) fn video_config_summary(
    config: &video_gen_config::Model,
) -> Result<VideoConfigSummary, String> {
    Ok(VideoConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config
            .base_url
            .clone()
            .ok_or_else(|| format!("视频配置 #{} 缺少 base_url", config.id))?,
        has_api_key: has_configured_api_key(config.api_key_encrypted.as_deref()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        default_width: config.default_width,
        default_height: config.default_height,
        default_duration_secs: config.default_duration_secs,
        min_duration_secs: config.min_duration_secs,
        max_duration_secs: config.max_duration_secs,
        duration_mode: a4n_api_contract::config::VideoDurationMode::from_str(&config.duration_mode)
            .map_err(|error| format!("视频配置 #{} 的 duration_mode 无效: {error}", config.id))?,
        supported_durations_json: config.supported_durations_json.clone(),
        default_fps: config.default_fps,
        is_default: config.is_default,
    })
}

#[cfg(test)]
mod tests {
    use a4n_api_contract::config::UpdateVideoConfigInput;
    use serde_json::Value;

    #[test]
    fn update_video_input_distinguishes_api_key_update_intents() {
        let base = serde_json::json!({
            "name": "video key",
            "backend": "openai-compatible-video",
            "base_url": "https://api.example/v1",
            "model": "video-model",
            "max_concurrent": 1,
            "default_width": 1280,
            "default_height": 720,
            "default_duration_secs": 6,
            "min_duration_secs": 1,
            "max_duration_secs": 60,
            "duration_mode": "range",
            "supported_durations_json": null,
            "default_fps": 24,
        });
        let missing_key = serde_json::from_value::<UpdateVideoConfigInput>(base.clone())
            .expect("missing api_key_encrypted should preserve the old key");
        assert_eq!(missing_key.api_key_encrypted, None);
        let mut clear_value = base.clone();
        clear_value["api_key_encrypted"] = Value::Null;
        let clear_key = serde_json::from_value::<UpdateVideoConfigInput>(clear_value)
            .expect("null api_key_encrypted should be an explicit clear request");
        assert_eq!(clear_key.api_key_encrypted, Some(None));
        let mut replace_value = base;
        replace_value["api_key_encrypted"] = Value::String("sk-new".to_string());
        let replace_key = serde_json::from_value::<UpdateVideoConfigInput>(replace_value)
            .expect("string api_key_encrypted should replace the old key");
        assert_eq!(
            replace_key.api_key_encrypted,
            Some(Some("sk-new".to_string()))
        );
    }
}
