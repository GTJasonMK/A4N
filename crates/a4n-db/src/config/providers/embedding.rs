use a4n_core::{error::AppError, models::ai::embedding_config};
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
