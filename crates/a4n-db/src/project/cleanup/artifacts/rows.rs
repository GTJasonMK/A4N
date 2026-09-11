use super::super::{ProjectCleanupDbError, ProjectGeneratedArtifactRowCleanupResult};
use crate::project::validate_positive_i64;
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    asset::profile_primary_material,
    generation::generation_spec,
    media::media_asset,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

pub(crate) async fn delete_project_generated_artifact_rows<C>(
    db: &C,
    project_id: i64,
    generated_image_ids: &[i64],
    generated_video_ids: &[i64],
    media_asset_ids: &[i64],
    generation_spec_ids: &[i64],
) -> Result<ProjectGeneratedArtifactRowCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let generated_video_refs = if generated_video_ids.is_empty() {
        Vec::new()
    } else {
        let mut query = generated_video::Entity::find()
            .select_only()
            .column(generated_video::Column::Id)
            .column(generated_video::Column::ProjectId)
            .filter(generated_video::Column::ProjectId.eq(project_id))
            .filter(generated_video::Column::Id.is_in(generated_video_ids.iter().copied()))
            .order_by_asc(generated_video::Column::Id);
        if db.get_database_backend() != DbBackend::Sqlite {
            query = query.lock_exclusive();
        }
        query.into_tuple::<(i64, i64)>().all(db).await?
    };

    crate::video::clear_generated_image_references_record(db, generated_image_ids).await?;
    if !generated_image_ids.is_empty() {
        profile_primary_material::Entity::delete_many()
            .filter(
                profile_primary_material::Column::PrimaryImageId
                    .is_in(generated_image_ids.iter().copied()),
            )
            .exec(db)
            .await?;
    }
    crate::video::clear_generated_video_references_by_ids(db, &generated_video_refs).await?;
    crate::media::delete_generated_image_target_media_bindings(db, generated_image_ids).await?;
    crate::media::delete_generated_video_target_media_bindings(db, generated_video_ids).await?;

    let deleted_generation_specs = if generation_spec_ids.is_empty() {
        0
    } else {
        generation_spec::Entity::delete_many()
            .filter(generation_spec::Column::ProjectId.eq(project_id))
            .filter(generation_spec::Column::Id.is_in(generation_spec_ids.to_vec()))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_media_assets = if media_asset_ids.is_empty() {
        0
    } else {
        media_asset::Entity::delete_many()
            .filter(media_asset::Column::ProjectId.eq(project_id))
            .filter(media_asset::Column::Id.is_in(media_asset_ids.to_vec()))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_generated_images = if generated_image_ids.is_empty() {
        0
    } else {
        generated_image::Entity::delete_many()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::Id.is_in(generated_image_ids.to_vec()))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_generated_videos = if generated_video_ids.is_empty() {
        0
    } else {
        generated_video::Entity::delete_many()
            .filter(generated_video::Column::ProjectId.eq(project_id))
            .filter(generated_video::Column::Id.is_in(generated_video_ids.to_vec()))
            .exec(db)
            .await?
            .rows_affected
    };

    Ok(ProjectGeneratedArtifactRowCleanupResult {
        deleted_generation_specs,
        deleted_media_assets,
        deleted_generated_images,
        deleted_generated_videos,
    })
}
