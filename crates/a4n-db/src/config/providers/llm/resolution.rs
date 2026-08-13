use a4n_core::{error::AppError, models::ai::llm_config};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::records::llm_config_by_id;

pub async fn default_llm_config(db: &DatabaseConnection) -> Result<llm_config::Model, AppError> {
    llm_config::Entity::find()
        .filter(llm_config::Column::IsDefault.eq(true))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::ConfigError("请先设置默认 LLM 配置".to_string()))
}

pub async fn resolve_llm_config(
    db: &DatabaseConnection,
    config_id: Option<i64>,
) -> Result<llm_config::Model, AppError> {
    match config_id {
        Some(id) => llm_config_by_id(db, id).await,
        None => default_llm_config(db).await,
    }
}
