use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_image, project::novel_project};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

pub(super) async fn generated_image_for_delete<C>(
    db: &C,
    image_id: i64,
) -> Result<generated_image::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let candidate = generated_image::Entity::find_by_id(image_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("图片不存在".to_string()))?;
    if db.get_database_backend() == DbBackend::Sqlite {
        return Ok(candidate);
    }

    novel_project::Entity::find_by_id(candidate.project_id)
        .lock_exclusive()
        .one(db)
        .await?;
    let locked = generated_image::Entity::find_by_id(image_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("图片不存在".to_string()))?;
    if locked.project_id != candidate.project_id {
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
        novel_project::Entity::find_by_id(project_id)
            .lock_exclusive()
            .one(db)
            .await?;
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
