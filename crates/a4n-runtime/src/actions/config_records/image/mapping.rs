use super::super::shared::has_configured_api_key;
use a4n_api_contract::config::ImageConfigSummary;
use a4n_core::models::ai::image_gen_config;

pub(super) fn image_config_summary(
    config: &image_gen_config::Model,
) -> Result<ImageConfigSummary, String> {
    Ok(ImageConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config
            .base_url
            .clone()
            .ok_or_else(|| format!("图片配置 #{} 缺少 base_url", config.id))?,
        has_api_key: has_configured_api_key(config.api_key_encrypted.as_deref()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        default_width: config.default_width,
        default_height: config.default_height,
        is_default: config.is_default,
    })
}

#[cfg(test)]
mod tests {
    use a4n_api_contract::config::UpdateImageConfigInput;
    use serde_json::Value;

    #[test]
    fn update_image_input_distinguishes_api_key_update_intents() {
        let base = serde_json::json!({
            "name": "image key",
            "backend": "openai-compatible-image",
            "base_url": "https://api.example/v1",
            "model": "image-model",
            "max_concurrent": 1,
            "default_width": 1024,
            "default_height": 1024,
        });
        let missing_key = serde_json::from_value::<UpdateImageConfigInput>(base.clone())
            .expect("missing api_key_encrypted should preserve the old key");
        assert_eq!(missing_key.api_key_encrypted, None);

        let mut clear_value = base.clone();
        clear_value["api_key_encrypted"] = Value::Null;
        let clear_key = serde_json::from_value::<UpdateImageConfigInput>(clear_value)
            .expect("null api_key_encrypted should be an explicit clear request");
        assert_eq!(clear_key.api_key_encrypted, Some(None));

        let mut replace_value = base;
        replace_value["api_key_encrypted"] = Value::String("sk-new".to_string());
        let replace_key = serde_json::from_value::<UpdateImageConfigInput>(replace_value)
            .expect("string api_key_encrypted should replace the old key");
        assert_eq!(
            replace_key.api_key_encrypted,
            Some(Some("sk-new".to_string()))
        );
    }
}
