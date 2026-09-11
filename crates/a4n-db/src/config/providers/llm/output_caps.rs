use super::contracts::DefaultLlmOutputCapRow;
use a4n_db_models::models::ai::llm_config;
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub async fn default_llm_output_cap_rows<C>(db: &C) -> Result<Vec<DefaultLlmOutputCapRow>, AppError>
where
    C: ConnectionTrait,
{
    Ok(llm_config::Entity::find()
        .filter(llm_config::Column::IsDefault.eq(true))
        .all(db)
        .await
        .map_err(AppError::Database)?
        .into_iter()
        .map(|model| DefaultLlmOutputCapRow {
            id: model.id,
            name: model.name,
            max_output_tokens: model.max_output_tokens,
        })
        .collect())
}
