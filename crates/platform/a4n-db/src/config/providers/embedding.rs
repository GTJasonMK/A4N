use a4n_db_models::models::ai::embedding_config;
use a4n_domain::embedding::{CreateEmbeddingConfigParams, validate_embedding_config_params};
use a4n_error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, Set,
};

pub struct EmbeddingConfigRecordInput {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
    pub pricing_json: serde_json::Value,
}

pub fn validate_embedding_config_model(
    model: &embedding_config::Model,
) -> Result<CreateEmbeddingConfigParams, AppError> {
    validate_embedding_config_params(CreateEmbeddingConfigParams {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        dimension: model.dimension,
        batch_size: model.batch_size,
        local_model_path: model.local_model_path.clone(),
    })
    .map_err(AppError::InvalidArgument)
}

pub async fn default_embedding_config_optional(
    db: &DatabaseConnection,
) -> Result<Option<embedding_config::Model>, AppError> {
    embedding_config::Entity::find()
        .filter(embedding_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)
}

pub async fn default_embedding_config(
    db: &DatabaseConnection,
) -> Result<embedding_config::Model, AppError> {
    default_embedding_config_optional(db)
        .await?
        .ok_or_else(|| AppError::ConfigError("请先设置默认 Embedding 配置".to_string()))
}

pub async fn list_embedding_configs<C>(db: &C) -> Result<Vec<embedding_config::Model>, AppError>
where
    C: ConnectionTrait,
{
    embedding_config::Entity::find()
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn embedding_config_by_id<C>(
    db: &C,
    config_id: i64,
) -> Result<embedding_config::Model, AppError>
where
    C: ConnectionTrait,
{
    embedding_config::Entity::find_by_id(config_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("嵌入配置不存在: {config_id}")))
}

pub async fn create_embedding_config_record<C>(
    db: &C,
    input: EmbeddingConfigRecordInput,
) -> Result<embedding_config::Model, AppError>
where
    C: ConnectionTrait,
{
    ensure_embedding_config_name_available(db, &input.name, None).await?;
    insert_embedding_config_record(db, input, false).await
}

pub(crate) async fn insert_embedding_config_record<C>(
    db: &C,
    input: EmbeddingConfigRecordInput,
    is_default: bool,
) -> Result<embedding_config::Model, AppError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    embedding_config::ActiveModel {
        name: Set(input.name),
        backend: Set(input.backend),
        model: Set(input.model),
        dimension: Set(input.dimension),
        batch_size: Set(input.batch_size),
        base_url: Set(input.base_url),
        api_key_encrypted: Set(input.api_key_encrypted),
        local_model_path: Set(input.local_model_path),
        pricing_json: Set(input.pricing_json),
        is_default: Set(is_default),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub async fn update_embedding_config_record<C>(
    db: &C,
    existing: embedding_config::Model,
    input: EmbeddingConfigRecordInput,
) -> Result<embedding_config::Model, AppError>
where
    C: ConnectionTrait,
{
    ensure_embedding_config_name_available(db, &input.name, Some(existing.id)).await?;

    let mut active: embedding_config::ActiveModel = existing.into();
    active.name = Set(input.name);
    active.backend = Set(input.backend);
    active.model = Set(input.model);
    active.dimension = Set(input.dimension);
    active.batch_size = Set(input.batch_size);
    active.base_url = Set(input.base_url);
    active.api_key_encrypted = Set(input.api_key_encrypted);
    active.local_model_path = Set(input.local_model_path);
    active.pricing_json = Set(input.pricing_json);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

pub async fn delete_embedding_config_record<C>(db: &C, config_id: i64) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    embedding_config::Entity::delete_by_id(config_id)
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn ensure_embedding_config_name_available<C>(
    db: &C,
    name: &str,
    except_id: Option<i64>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let mut query =
        embedding_config::Entity::find().filter(embedding_config::Column::Name.eq(name));
    if let Some(config_id) = except_id {
        query = query.filter(embedding_config::Column::Id.ne(config_id));
    }
    let count = query.count(db).await.map_err(AppError::Database)?;
    if count > 0 {
        return Err(AppError::InvalidArgument(format!(
            "嵌入配置名称已存在: {name}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::embedding::{EMBEDDING_PROVIDER_CAPABILITIES, EmbeddingBackendKind};

    fn backend(kind: EmbeddingBackendKind) -> &'static str {
        EMBEDDING_PROVIDER_CAPABILITIES
            .iter()
            .find(|capabilities| capabilities.kind == kind)
            .expect("embedding backend capability")
            .backend
    }

    fn model(backend: &str) -> embedding_config::Model {
        embedding_config::Model {
            id: 1,
            name: "  默认嵌入  ".to_string(),
            backend: backend.to_string(),
            base_url: None,
            api_key_encrypted: Some("  secret  ".to_string()),
            model: "  embedding-model  ".to_string(),
            dimension: 384,
            batch_size: 32,
            local_model_path: Some("  /models/embedding  ".to_string()),
            pricing_json: serde_json::json!({}),
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
        let mut model = model(backend(EmbeddingBackendKind::LocalHash));
        let params = validate_embedding_config_model(&model).expect("valid embedding model");
        assert_eq!(params.name, "默认嵌入");
        assert_eq!(params.backend, backend(EmbeddingBackendKind::LocalHash));
        assert_eq!(params.base_url, None);
        assert_eq!(params.api_key_encrypted, None);
        assert_eq!(params.model, "embedding-model");
        assert_eq!(params.dimension, 384);
        assert_eq!(params.batch_size, 32);
        assert_eq!(
            params.local_model_path.as_deref(),
            Some("/models/embedding")
        );

        model.backend = "unsupported-embedding-backend".to_string();
        assert_invalid_argument(
            validate_embedding_config_model(&model).expect_err("unknown embedding backend"),
            "不支持的嵌入后端: unsupported-embedding-backend",
        );

        model.backend = backend(EmbeddingBackendKind::Api).to_string();
        assert_invalid_argument(
            validate_embedding_config_model(&model).expect_err("missing embedding base url"),
            "api 嵌入后端必须填写 base_url",
        );

        model.backend = backend(EmbeddingBackendKind::FastEmbed).to_string();
        model.dimension = 0;
        assert_invalid_argument(
            validate_embedding_config_model(&model).expect_err("invalid embedding dimension"),
            "嵌入配置 dimension 必须大于 0",
        );
    }

    #[test]
    fn orm_model_conversion_rejects_missing_model_with_domain_error() {
        let mut model = model(backend(EmbeddingBackendKind::LocalHash));
        model.model = "  ".to_string();

        assert_invalid_argument(
            validate_embedding_config_model(&model).expect_err("missing embedding model"),
            "嵌入配置缺少 model",
        );
    }
}
