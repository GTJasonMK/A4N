use super::contracts::ImageConfigRecordInput;
use a4n_core::{error::AppError, models::ai::image_gen_config};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

pub async fn list_image_config_records<C>(db: &C) -> Result<Vec<image_gen_config::Model>, AppError>
where
    C: ConnectionTrait,
{
    image_gen_config::Entity::find()
        .order_by_asc(image_gen_config::Column::Name)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn image_config_by_id<C>(
    db: &C,
    config_id: i64,
) -> Result<image_gen_config::Model, AppError>
where
    C: ConnectionTrait,
{
    image_gen_config::Entity::find_by_id(config_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("图片配置 #{config_id} 不存在")))
}

pub async fn create_image_config_record<C>(
    db: &C,
    input: ImageConfigRecordInput,
) -> Result<image_gen_config::Model, AppError>
where
    C: ConnectionTrait,
{
    ensure_image_config_name_available(db, &input.name, None).await?;
    let now = Utc::now().naive_utc();
    image_gen_config::ActiveModel {
        name: Set(input.name),
        backend: Set(input.backend),
        base_url: Set(Some(input.base_url)),
        api_key_encrypted: Set(input.api_key_encrypted),
        model: Set(input.model),
        max_concurrent: Set(input.max_concurrent),
        default_width: Set(input.default_width),
        default_height: Set(input.default_height),
        is_default: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub async fn update_image_config_record<C>(
    db: &C,
    existing: image_gen_config::Model,
    input: ImageConfigRecordInput,
) -> Result<image_gen_config::Model, AppError>
where
    C: ConnectionTrait,
{
    ensure_image_config_name_available(db, &input.name, Some(existing.id)).await?;

    let mut active: image_gen_config::ActiveModel = existing.into();
    active.name = Set(input.name);
    active.backend = Set(input.backend);
    active.base_url = Set(Some(input.base_url));
    active.api_key_encrypted = Set(input.api_key_encrypted);
    active.model = Set(input.model);
    active.max_concurrent = Set(input.max_concurrent);
    active.default_width = Set(input.default_width);
    active.default_height = Set(input.default_height);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

pub async fn delete_image_config_record<C>(db: &C, config_id: i64) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    image_gen_config::Entity::delete_by_id(config_id)
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn ensure_image_config_name_available<C>(
    db: &C,
    name: &str,
    except_id: Option<i64>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let mut query =
        image_gen_config::Entity::find().filter(image_gen_config::Column::Name.eq(name));
    if let Some(config_id) = except_id {
        query = query.filter(image_gen_config::Column::Id.ne(config_id));
    }
    let count = query.count(db).await.map_err(AppError::Database)?;
    if count > 0 {
        return Err(AppError::InvalidArgument(format!(
            "图片配置名称已存在: {name}"
        )));
    }
    Ok(())
}
