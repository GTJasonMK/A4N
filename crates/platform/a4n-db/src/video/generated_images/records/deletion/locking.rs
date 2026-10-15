use crate::video::VideoDbError;
use a4n_db_models::models::ai::generated_image;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};

#[derive(Debug, FromQueryResult)]
pub(super) struct GeneratedImageDeleteContext {
    pub(super) id: i64,
    pub(super) project_id: i64,
    pub(super) file_path: String,
}

pub(super) async fn generated_image_for_delete<C>(
    db: &C,
    image_id: i64,
) -> Result<GeneratedImageDeleteContext, VideoDbError>
where
    C: ConnectionTrait,
{
    let image_columns = |query: sea_orm::Select<generated_image::Entity>| {
        query
            .select_only()
            .column(generated_image::Column::Id)
            .column(generated_image::Column::ProjectId)
            .column(generated_image::Column::FilePath)
            .into_model::<GeneratedImageDeleteContext>()
    };
    if db.get_database_backend() == DbBackend::Sqlite {
        return image_columns(generated_image::Entity::find_by_id(image_id))
            .one(db)
            .await?
            .ok_or_else(|| VideoDbError::NotFound("图片不存在".to_string()));
    }
    let candidate_project_id = generated_image::Entity::find_by_id(image_id)
        .select_only()
        .column(generated_image::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("图片不存在".to_string()))?;
    crate::video::lock_generated_media_project(db, candidate_project_id).await?;
    let locked = image_columns(generated_image::Entity::find_by_id(image_id).lock_exclusive())
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("图片不存在".to_string()))?;
    if locked.project_id != candidate_project_id {
        return Err(VideoDbError::InvalidArgument(format!(
            "图片 {image_id} 的项目归属在删除期间发生变化，请重试"
        )));
    }
    Ok(locked)
}

pub(super) async fn generated_images_for_delete<C>(
    db: &C,
    project_id: i64,
    image_ids: &[i64],
) -> Result<Vec<generated_image::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }
    if db.get_database_backend() != DbBackend::Sqlite {
        crate::video::lock_generated_media_project(db, project_id).await?;
    }
    let mut query = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::Id.is_in(image_ids.iter().copied()))
        .order_by_asc(generated_image::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    Ok(query.all(db).await?)
}
