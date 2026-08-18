use super::existence::{
    embedding_configs_exist, image_configs_exist, llm_configs_exist, video_configs_exist,
};
use super::super::{
    EmbeddingConfigRecordInput, ImageConfigRecordInput, LlmConfigRecordInput,
    VideoConfigRecordInput, insert_embedding_config_record, insert_image_config_record,
    insert_llm_config_record, insert_video_config_record,
};
use a4n_core::error::AppError;
use sea_orm::DatabaseConnection;

pub async fn insert_baseline_embedding_config_if_empty(
    db: &DatabaseConnection,
) -> Result<bool, AppError> {
    if embedding_configs_exist(db).await? {
        return Ok(false);
    }

    insert_embedding_config_record(
        db,
        EmbeddingConfigRecordInput {
            name: "基础本地哈希向量".to_string(),
            backend: "local_hash".to_string(),
            base_url: None,
            api_key_encrypted: None,
            model: "local-sha256-hash".to_string(),
            dimension: 384,
            batch_size: 100,
            local_model_path: None,
        },
        true,
    )
    .await?;
    Ok(true)
}

pub async fn insert_baseline_llm_config_if_empty(
    db: &DatabaseConnection,
    input: LlmConfigRecordInput,
) -> Result<bool, AppError> {
    if llm_configs_exist(db).await? {
        return Ok(false);
    }

    insert_llm_config_record(db, input, true).await?;
    Ok(true)
}

pub async fn insert_baseline_image_config_if_empty(
    db: &DatabaseConnection,
    input: ImageConfigRecordInput,
) -> Result<bool, AppError> {
    if image_configs_exist(db).await? {
        return Ok(false);
    }

    insert_image_config_record(db, input, true).await?;
    Ok(true)
}

pub async fn insert_baseline_video_config_if_empty(
    db: &DatabaseConnection,
    input: VideoConfigRecordInput,
) -> Result<bool, AppError> {
    if video_configs_exist(db).await? {
        return Ok(false);
    }

    insert_video_config_record(db, input, true).await?;
    Ok(true)
}
