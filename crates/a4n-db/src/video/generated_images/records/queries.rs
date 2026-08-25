use super::super::{VideoDbError, validate_positive_video_i64, validate_positive_video_ids};
use a4n_core::models::ai::generated_image;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

pub async fn generated_images_for_video_prompt_references(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<generated_image::Reference>, VideoDbError> {
    validate_positive_video_i64(project_id, "project_id")?;
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::Id)
        .into_model::<generated_image::Reference>()
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn generated_images_by_ids(
    db: &DatabaseConnection,
    image_ids: &[i64],
) -> Result<Vec<generated_image::Model>, VideoDbError> {
    validate_positive_video_ids(image_ids, "image_ids")?;
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }
    generated_image::Entity::find()
        .filter(generated_image::Column::Id.is_in(image_ids.iter().copied()))
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn generated_image_ids_by_ids<C>(
    db: &C,
    image_ids: &[i64],
) -> Result<Vec<i64>, VideoDbError>
where
    C: ConnectionTrait,
{
    validate_positive_video_ids(image_ids, "image_ids")?;
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }
    generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .filter(generated_image::Column::Id.is_in(image_ids.iter().copied()))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub(crate) async fn generated_images_for_chapter_by_id_desc<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<generated_image::Reference>, DbErr>
where
    C: ConnectionTrait,
{
    generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::Id)
        .into_model::<generated_image::Reference>()
        .all(db)
        .await
}

pub async fn generated_images_for_projects_chapter_by_id_desc<C>(
    db: &C,
    project_ids: &[i64],
    chapter_id: i64,
) -> Result<Vec<generated_image::Reference>, DbErr>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::ImageType)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::Id)
        .into_model::<generated_image::Reference>()
        .all(db)
        .await
}

pub(in crate::video::generated_images) async fn generated_image_records_by_ids<C>(
    db: &C,
    image_ids: &[i64],
) -> Result<Vec<generated_image::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }
    generated_image::Entity::find()
        .filter(generated_image::Column::Id.is_in(image_ids.iter().copied()))
        .all(db)
        .await
}

pub async fn generated_images_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<generated_image::Model>, DbErr>
where
    C: ConnectionTrait,
{
    generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .order_by_desc(generated_image::Column::CreatedAt)
        .all(db)
        .await
}

pub async fn generated_images_for_projects<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<generated_image::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    validate_positive_video_ids(project_ids, "project_ids")?;
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_desc(generated_image::Column::CreatedAt)
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn generated_images_for_chapter<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<generated_image::Model>, DbErr>
where
    C: ConnectionTrait,
{
    generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::CreatedAt)
        .all(db)
        .await
}

pub async fn generated_image_by_id<C>(
    db: &C,
    image_id: i64,
) -> Result<Option<generated_image::Model>, DbErr>
where
    C: ConnectionTrait,
{
    generated_image::Entity::find_by_id(image_id).one(db).await
}

pub async fn generated_image_exists<C>(db: &C, image_id: i64) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    Ok(generated_image::Entity::find_by_id(image_id)
        .select_only()
        .column(generated_image::Column::Id)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some())
}
