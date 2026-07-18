use super::contracts::{ImageConfigInput, ImageConfigSummary, UpdateImageConfigInput};
use a4n_core::{
    config::{CreateImageConfigParams, UpdateImageConfigParams, validate_image_config_params},
    models::ai::image_gen_config,
};
use a4n_orchestrator::config::{ImageConfigCreateRecordInput, ImageConfigUpdateRecordInput};

pub(super) fn image_config_summary(config: &image_gen_config::Model) -> ImageConfigSummary {
    ImageConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config.base_url.clone(),
        api_key_encrypted: None,
        has_api_key: config
            .api_key_encrypted
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        model: config.model.clone(),
        max_concurrent: config.max_concurrent,
        default_width: config.default_width,
        default_height: config.default_height,
        provider_options_json: config.provider_options_json.clone(),
        is_default: config.is_default,
        created_at: Some(config.created_at.to_string()),
        updated_at: Some(config.updated_at.to_string()),
    }
}

pub(super) fn image_create_record_input(
    input: ImageConfigInput,
) -> Result<ImageConfigCreateRecordInput, String> {
    let params =
        validate_image_config_params(image_config_params(input)).map_err(|err| err.to_string())?;
    Ok(ImageConfigCreateRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        default_width: params.default_width,
        default_height: params.default_height,
        provider_options_json: params.provider_options_json,
    })
}

pub(super) fn image_update_record_input(
    existing: &image_gen_config::Model,
    input: UpdateImageConfigInput,
) -> Result<ImageConfigUpdateRecordInput, String> {
    let params = image_update_config_params(input);
    let api_key_encrypted = params
        .api_key_encrypted
        .clone()
        .unwrap_or_else(|| existing.api_key_encrypted.clone());
    let params = validate_image_config_params(CreateImageConfigParams {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        default_width: params.default_width,
        default_height: params.default_height,
        provider_options_json: params.provider_options_json,
    })
    .map_err(|err| err.to_string())?;
    Ok(ImageConfigUpdateRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        max_concurrent: params.max_concurrent,
        default_width: params.default_width,
        default_height: params.default_height,
        provider_options_json: params.provider_options_json,
    })
}

fn image_config_params(input: ImageConfigInput) -> CreateImageConfigParams {
    CreateImageConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        default_width: input.default_width,
        default_height: input.default_height,
        provider_options_json: input.provider_options_json,
    }
}

fn image_update_config_params(input: UpdateImageConfigInput) -> UpdateImageConfigParams {
    UpdateImageConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        max_concurrent: input.max_concurrent,
        default_width: input.default_width,
        default_height: input.default_height,
        provider_options_json: input.provider_options_json,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            "provider_options_json": null
        });
        let missing_key = serde_json::from_value::<UpdateImageConfigInput>(base.clone())
            .expect("missing api_key_encrypted should preserve the old key");
        assert_eq!(missing_key.api_key_encrypted, None);
        assert_eq!(
            image_update_config_params(missing_key).api_key_encrypted,
            None
        );

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
            image_update_config_params(replace_key).api_key_encrypted,
            Some(Some("sk-new".to_string()))
        );
    }
}
