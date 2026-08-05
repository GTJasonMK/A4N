use a4n_core::{error::AppError, models::ai::video_gen_config};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};

use super::contracts::{VideoConfigCreateRecordInput, VideoConfigUpdateRecordInput};

pub async fn list_video_config_records<C>(db: &C) -> Result<Vec<video_gen_config::Model>, AppError>
where
    C: ConnectionTrait,
{
    video_gen_config::Entity::find()
        .order_by_asc(video_gen_config::Column::Name)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn video_config_record_by_id<C>(
    db: &C,
    config_id: i64,
) -> Result<video_gen_config::Model, AppError>
where
    C: ConnectionTrait,
{
    video_gen_config::Entity::find_by_id(config_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("视频配置不存在".to_string()))
}

pub async fn create_video_config_record<C>(
    db: &C,
    input: VideoConfigCreateRecordInput,
) -> Result<video_gen_config::Model, AppError>
where
    C: ConnectionTrait,
{
    ensure_video_config_name_available(db, &input.name, None).await?;
    let now = Utc::now().naive_utc();
    video_gen_config::ActiveModel {
        name: Set(input.name),
        backend: Set(input.backend),
        base_url: Set(Some(input.base_url)),
        api_key_encrypted: Set(input.api_key_encrypted),
        model: Set(input.model),
        max_concurrent: Set(input.max_concurrent),
        default_width: Set(input.default_width),
        default_height: Set(input.default_height),
        default_duration_secs: Set(input.default_duration_secs),
        min_duration_secs: Set(input.min_duration_secs),
        max_duration_secs: Set(input.max_duration_secs),
        duration_mode: Set(input.duration_mode),
        supported_durations_json: Set(input.supported_durations_json),
        default_fps: Set(input.default_fps),
        is_default: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}

pub async fn update_video_config_record<C>(
    db: &C,
    existing: video_gen_config::Model,
    input: VideoConfigUpdateRecordInput,
) -> Result<video_gen_config::Model, AppError>
where
    C: ConnectionTrait,
{
    ensure_video_config_name_available(db, &input.name, Some(existing.id)).await?;

    let mut active: video_gen_config::ActiveModel = existing.into();
    active.name = Set(input.name);
    active.backend = Set(input.backend);
    active.base_url = Set(Some(input.base_url));
    active.api_key_encrypted = Set(input.api_key_encrypted);
    active.model = Set(input.model);
    active.max_concurrent = Set(input.max_concurrent);
    active.default_width = Set(input.default_width);
    active.default_height = Set(input.default_height);
    active.default_duration_secs = Set(input.default_duration_secs);
    active.min_duration_secs = Set(input.min_duration_secs);
    active.max_duration_secs = Set(input.max_duration_secs);
    active.duration_mode = Set(input.duration_mode);
    active.supported_durations_json = Set(input.supported_durations_json);
    active.default_fps = Set(input.default_fps);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

pub async fn set_default_video_config_record(
    db: &DatabaseConnection,
    model: video_gen_config::Model,
) -> Result<video_gen_config::Model, AppError> {
    let txn = db.begin().await.map_err(AppError::Database)?;
    video_gen_config::Entity::update_many()
        .col_expr(
            video_gen_config::Column::IsDefault,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(video_gen_config::Column::IsDefault.eq(true))
        .exec(&txn)
        .await
        .map_err(AppError::Database)?;

    let mut active: video_gen_config::ActiveModel = model.into();
    active.is_default = Set(true);
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&txn).await.map_err(AppError::Database)?;
    txn.commit().await.map_err(AppError::Database)?;
    Ok(updated)
}

pub async fn delete_video_config_record<C>(db: &C, config_id: i64) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    video_gen_config::Entity::delete_by_id(config_id)
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

async fn ensure_video_config_name_available<C>(
    db: &C,
    name: &str,
    except_id: Option<i64>,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let mut query =
        video_gen_config::Entity::find().filter(video_gen_config::Column::Name.eq(name));
    if let Some(config_id) = except_id {
        query = query.filter(video_gen_config::Column::Id.ne(config_id));
    }
    let count = query.count(db).await.map_err(AppError::Database)?;
    if count > 0 {
        return Err(AppError::InvalidArgument(format!(
            "视频配置名称已存在: {name}"
        )));
    }
    Ok(())
}
