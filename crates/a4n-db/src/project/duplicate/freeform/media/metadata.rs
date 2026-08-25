use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::FreeformCopyMaps;
use crate::project::duplicate::freeform::remap::remap_freeform_metadata;
use crate::project::duplicate::shared::remap_derived_required_id;
use a4n_core::models::ai::{generated_image, generated_video};
use chrono::NaiveDateTime;
use sea_orm::sea_query::{Alias, CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

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
        .select_only()
        .column(generated_image::Column::Id)
        .column(generated_image::Column::MetadataJson)
        .filter(generated_image::Column::ProjectId.eq(source_project_id))
        .into_tuple::<(i64, Option<serde_json::Value>)>()
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut image_metadata_case = CaseStatement::new();
    let mut image_ids = Vec::new();
    for (image_id, metadata_json) in images {
        let Some(metadata) = metadata_json else {
            continue;
        };
        let metadata = remap_freeform_metadata(metadata, maps)?;
        let new_id = remap_derived_required_id(&maps.images.ids, image_id, "generated_image.id")?;
        image_metadata_case = image_metadata_case.case(
            generated_image::Column::Id.eq(new_id),
            Expr::value(Some(metadata)).cast_as(Alias::new("json")),
        );
        image_ids.push(new_id);
    }
    if !image_ids.is_empty() {
        let updated = generated_image::Entity::update_many()
            .col_expr(
                generated_image::Column::MetadataJson,
                image_metadata_case
                    .finally(Expr::col(generated_image::Column::MetadataJson))
                    .into(),
            )
            .filter(generated_image::Column::Id.is_in(image_ids.clone()))
            .exec(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
        if updated.rows_affected != image_ids.len() as u64 {
            return Err(ProjectDuplicateError::InvalidState {
                current: "复制后的生成图片元数据数量与源记录不一致".to_string(),
                operation: "回填生成图片元数据".to_string(),
            });
        }
    }

    let videos = generated_video::Entity::find()
        .select_only()
        .column(generated_video::Column::Id)
        .column(generated_video::Column::MetadataJson)
        .filter(generated_video::Column::ProjectId.eq(source_project_id))
        .into_tuple::<(i64, Option<serde_json::Value>)>()
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut video_metadata_case = CaseStatement::new();
    let mut video_ids = Vec::new();
    for (video_id, metadata_json) in videos {
        let Some(metadata) = metadata_json else {
            continue;
        };
        let metadata = remap_freeform_metadata(metadata, maps)?;
        let new_id = remap_derived_required_id(&maps.videos.ids, video_id, "generated_video.id")?;
        video_metadata_case = video_metadata_case.case(
            generated_video::Column::Id.eq(new_id),
            Expr::value(Some(metadata)).cast_as(Alias::new("json")),
        );
        video_ids.push(new_id);
    }
    if !video_ids.is_empty() {
        let updated = generated_video::Entity::update_many()
            .col_expr(
                generated_video::Column::MetadataJson,
                video_metadata_case
                    .finally(Expr::col(generated_video::Column::MetadataJson))
                    .into(),
            )
            .col_expr(generated_video::Column::UpdatedAt, Expr::value(now))
            .filter(generated_video::Column::Id.is_in(video_ids.clone()))
            .exec(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
        if updated.rows_affected != video_ids.len() as u64 {
            return Err(ProjectDuplicateError::InvalidState {
                current: "复制后的生成视频元数据数量与源记录不一致".to_string(),
                operation: "回填生成视频元数据".to_string(),
            });
        }
    }
    Ok(())
}
