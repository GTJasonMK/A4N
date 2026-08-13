use super::contracts::LlmConfigRecordInput;
use a4n_core::{error::AppError, models::ai::llm_config};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

pub async fn list_llm_config_records<C>(db: &C) -> Result<Vec<llm_config::Model>, AppError>
where
    C: ConnectionTrait,
{
    llm_config::Entity::find()
        .order_by_desc(llm_config::Column::UpdatedAt)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn llm_config_by_id<C>(db: &C, config_id: i64) -> Result<llm_config::Model, AppError>
where
    C: ConnectionTrait,
{
    llm_config::Entity::find_by_id(config_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("LLM配置不存在: {config_id}")))
}

pub async fn create_llm_config_record<C>(
    db: &C,
    input: LlmConfigRecordInput,
) -> Result<llm_config::Model, AppError>
where
    C: ConnectionTrait,
{
    ensure_llm_config_name_available(db, &input.name, None).await?;
    let now = Utc::now().naive_utc();
    llm_config::ActiveModel {
        name: Set(input.name),
        provider: Set(input.provider),
        base_url: Set(input.base_url),
        api_key_encrypted: Set(input.api_key_encrypted),
        model: Set(input.model),
        max_concurrent: Set(input.max_concurrent),
        timeout_secs: Set(input.timeout_secs),
        max_context_tokens: Set(input.max_context_tokens),
        max_output_tokens: Set(input.max_output_tokens),
        extra_body_json: Set(input.extra_body_json),
        is_default: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub async fn update_llm_config_record<C>(
    db: &C,
    existing: llm_config::Model,
    input: LlmConfigRecordInput,
) -> Result<llm_config::Model, AppError>
where
    C: ConnectionTrait,
{
    ensure_llm_config_name_available(db, &input.name, Some(existing.id)).await?;

    let mut active: llm_config::ActiveModel = existing.into();
    active.name = Set(input.name);
    active.provider = Set(input.provider);
    active.base_url = Set(input.base_url);
    active.api_key_encrypted = Set(input.api_key_encrypted);
    active.model = Set(input.model);
    active.max_concurrent = Set(input.max_concurrent);
    active.timeout_secs = Set(input.timeout_secs);
    active.max_context_tokens = Set(input.max_context_tokens);
    active.max_output_tokens = Set(input.max_output_tokens);
    active.extra_body_json = Set(input.extra_body_json);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

pub async fn delete_llm_config_record<C>(db: &C, config_id: i64) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    llm_config::Entity::delete_by_id(config_id)
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn ensure_llm_config_name_available<C>(
    db: &C,
    name: &str,
    except_id: Option<i64>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let mut query = llm_config::Entity::find().filter(llm_config::Column::Name.eq(name));
    if let Some(config_id) = except_id {
        query = query.filter(llm_config::Column::Id.ne(config_id));
    }
    let count = query.count(db).await.map_err(AppError::Database)?;
    if count > 0 {
        return Err(AppError::InvalidArgument(format!(
            "LLM配置名称已存在: {name}"
        )));
    }
    Ok(())
}
