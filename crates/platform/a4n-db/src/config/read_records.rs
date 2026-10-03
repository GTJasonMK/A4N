use a4n_db_models::models::ai::{embedding_config, image_gen_config, llm_config, video_gen_config};
use a4n_domain::config::{
    EmbeddingConfigReadRecord, ImageConfigReadRecord, LlmConfigReadRecord, VideoConfigReadRecord,
};
use a4n_error::AppError;
use sea_orm::DatabaseConnection;

/// Host-facing provider reads. The ORM row is converted inside the database
/// adapter so Runtime/Orchestrator only receive domain configuration records.
pub async fn resolve_llm_config_read_record(
    db: &DatabaseConnection,
    config_id: Option<i64>,
) -> Result<LlmConfigReadRecord, AppError> {
    super::resolve_llm_config(db, config_id)
        .await
        .map(|model| llm_config_read_record(&model))
}

pub async fn image_config_read_record_by_id(
    db: &DatabaseConnection,
    config_id: i64,
) -> Result<ImageConfigReadRecord, AppError> {
    super::image_config_by_id(db, config_id)
        .await
        .map(|model| image_config_read_record(&model))
}

pub async fn resolve_image_config_read_record(
    db: &DatabaseConnection,
    config_id: Option<i64>,
) -> Result<ImageConfigReadRecord, AppError> {
    super::resolve_image_config(db, config_id)
        .await
        .map(|model| image_config_read_record(&model))
}

pub async fn video_config_read_record_by_id(
    db: &DatabaseConnection,
    config_id: i64,
) -> Result<VideoConfigReadRecord, AppError> {
    super::video_config_by_id(db, config_id)
        .await
        .map(|model| video_config_read_record(&model))
}

pub async fn default_video_config_read_record(
    db: &DatabaseConnection,
) -> Result<VideoConfigReadRecord, AppError> {
    super::default_video_config(db)
        .await
        .map(|model| video_config_read_record(&model))
}

pub async fn default_embedding_config_read_record_optional(
    db: &DatabaseConnection,
) -> Result<Option<EmbeddingConfigReadRecord>, AppError> {
    super::default_embedding_config_optional(db)
        .await
        .map(|model| model.as_ref().map(embedding_config_read_record))
}

pub async fn default_embedding_config_read_record(
    db: &DatabaseConnection,
) -> Result<EmbeddingConfigReadRecord, AppError> {
    super::default_embedding_config(db)
        .await
        .map(|model| embedding_config_read_record(&model))
}

pub fn llm_config_read_record(model: &llm_config::Model) -> LlmConfigReadRecord {
    LlmConfigReadRecord {
        id: model.id,
        name: model.name.clone(),
        provider: model.provider.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        timeout_secs: model.timeout_secs,
        max_context_tokens: model.max_context_tokens,
        max_output_tokens: model.max_output_tokens,
        extra_body_json: model.extra_body_json.clone(),
        agent_protocol: model.agent_protocol.clone(),
        pricing_json: model.pricing_json.clone(),
        is_default: model.is_default,
    }
}

pub fn embedding_config_read_record(model: &embedding_config::Model) -> EmbeddingConfigReadRecord {
    EmbeddingConfigReadRecord {
        id: model.id,
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        dimension: model.dimension,
        batch_size: model.batch_size,
        local_model_path: model.local_model_path.clone(),
        pricing_json: model.pricing_json.clone(),
        is_default: model.is_default,
    }
}

pub fn image_config_read_record(model: &image_gen_config::Model) -> ImageConfigReadRecord {
    ImageConfigReadRecord {
        id: model.id,
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        supported_sizes_json: model.supported_sizes_json.clone(),
        pricing_json: model.pricing_json.clone(),
        is_default: model.is_default,
    }
}

pub fn video_config_read_record(model: &video_gen_config::Model) -> VideoConfigReadRecord {
    VideoConfigReadRecord {
        id: model.id,
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        default_width: model.default_width,
        default_height: model.default_height,
        default_duration_secs: model.default_duration_secs,
        min_duration_secs: model.min_duration_secs,
        max_duration_secs: model.max_duration_secs,
        duration_mode: model.duration_mode.clone(),
        supported_durations_json: model.supported_durations_json.clone(),
        default_fps: model.default_fps,
        is_default: model.is_default,
        pricing_json: model.pricing_json.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn image_record_keeps_provider_fields_used_by_host() {
        let model = image_gen_config::Model {
            id: 7,
            name: "image".into(),
            backend: "openai-compatible-image".into(),
            base_url: Some("https://example.test/v1".into()),
            api_key_encrypted: Some("secret".into()),
            model: "image-model".into(),
            max_concurrent: 3,
            supported_sizes_json: json!([[1024, 1024]]),
            pricing_json: json!({}),
            is_default: true,
            created_at: Default::default(),
            updated_at: Default::default(),
        };

        assert_eq!(
            image_config_read_record(&model),
            ImageConfigReadRecord {
                id: 7,
                name: "image".into(),
                backend: "openai-compatible-image".into(),
                base_url: Some("https://example.test/v1".into()),
                api_key_encrypted: Some("secret".into()),
                model: "image-model".into(),
                max_concurrent: 3,
                supported_sizes_json: json!([[1024, 1024]]),
                pricing_json: json!({}),
                is_default: true,
            }
        );
    }
}
