use a4n_db_models::models::ai::image_gen_config;
use a4n_domain::image::{
    CreateImageConfigParams, image_sizes_from_json, validate_image_config_params,
};
use a4n_error::AppError;

pub fn validate_image_config_model(
    model: &image_gen_config::Model,
) -> Result<CreateImageConfigParams, AppError> {
    validate_image_config_params(CreateImageConfigParams {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model
            .base_url
            .clone()
            .ok_or_else(|| AppError::InvalidArgument("图片生成配置缺少 base_url".into()))?,
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        supported_sizes: image_sizes_from_json(&model.supported_sizes_json)
            .map_err(AppError::InvalidArgument)?,
    })
    .map_err(AppError::InvalidArgument)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::image::OPENAI_COMPATIBLE_IMAGE_BACKEND;

    fn model() -> image_gen_config::Model {
        image_gen_config::Model {
            id: 1,
            name: "image".to_string(),
            backend: OPENAI_COMPATIBLE_IMAGE_BACKEND.to_string(),
            base_url: Some("https://example.test/v1".to_string()),
            api_key_encrypted: Some("key".to_string()),
            model: "image-model".to_string(),
            max_concurrent: 1,
            supported_sizes_json: serde_json::json!([{"width": 1024, "height": 1024}]),
            is_default: true,
            created_at: chrono::NaiveDateTime::MIN,
            updated_at: chrono::NaiveDateTime::MIN,
        }
    }

    fn assert_invalid_argument(error: AppError, message: &str) {
        assert_eq!(error.to_string(), format!("参数无效：{message}"));
        assert!(std::error::Error::source(&error).is_none());
        assert!(matches!(
            error,
            AppError::InvalidArgument(actual) if actual == message
        ));
    }

    #[test]
    fn orm_model_conversion_maps_fields_and_reuses_validation() {
        let mut model = model();
        let params = validate_image_config_model(&model).expect("valid image model");
        assert_eq!(params.name, "image");
        assert_eq!(params.backend, OPENAI_COMPATIBLE_IMAGE_BACKEND);
        assert_eq!(params.base_url, "https://example.test/v1");
        assert_eq!(params.api_key_encrypted.as_deref(), Some("key"));
        assert_eq!(params.model, "image-model");
        assert_eq!(params.max_concurrent, 1);
        assert_eq!(
            params.supported_sizes,
            vec![a4n_domain::ImageSize::new(1024, 1024)]
        );

        model.backend = "unsupported-image-backend".to_string();
        assert_invalid_argument(
            validate_image_config_model(&model).expect_err("unknown image backend"),
            "未知图片生成后端: unsupported-image-backend",
        );

        model.backend = OPENAI_COMPATIBLE_IMAGE_BACKEND.to_string();
        model.base_url = None;
        assert_invalid_argument(
            validate_image_config_model(&model).expect_err("missing image base url"),
            "图片生成配置缺少 base_url",
        );

        model.base_url = Some("https://example.test/v1".to_string());
        model.supported_sizes_json = serde_json::json!([{"width": 0, "height": 1024}]);
        assert_invalid_argument(
            validate_image_config_model(&model).expect_err("invalid image size"),
            "图片配置 supported_sizes 包含无效尺寸 0x1024",
        );
    }

    #[test]
    fn orm_model_conversion_keeps_malformed_sizes_as_invalid_argument() {
        let mut model = model();
        model.supported_sizes_json = serde_json::json!({"width": 1024});

        let error = validate_image_config_model(&model).expect_err("malformed image sizes");
        assert!(matches!(
            error,
            AppError::InvalidArgument(ref message)
                if message.starts_with("图片配置 supported_sizes 无效:")
        ));
        assert!(std::error::Error::source(&error).is_none());
    }
}
