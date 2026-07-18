use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::FreeformCopyMaps;
use crate::project::duplicate::freeform::remap::remap_freeform_metadata;
use crate::project::duplicate::shared::remap_derived_required_id;
use a4n_core::models::ai::{generated_image, generated_video};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub(in crate::project::duplicate::freeform) async fn update_freeform_generated_media_metadata_record<
    C,
>(
    db: &C,
    source_project_id: i64,
    maps: &FreeformCopyMaps,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(source_project_id))
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    for image in images {
        let Some(metadata) = image.metadata_json else {
            continue;
        };
        let metadata = remap_freeform_metadata(metadata, maps)?;
        generated_image::ActiveModel {
            id: Set(remap_derived_required_id(
                &maps.images.ids,
                image.id,
                "generated_image.id",
            )?),
            metadata_json: Set(Some(metadata)),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }

    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(source_project_id))
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    for video in videos {
        let Some(metadata) = video.metadata_json else {
            continue;
        };
        let metadata = remap_freeform_metadata(metadata, maps)?;
        generated_video::ActiveModel {
            id: Set(remap_derived_required_id(
                &maps.videos.ids,
                video.id,
                "generated_video.id",
            )?),
            metadata_json: Set(Some(metadata)),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }
    Ok(())
}
