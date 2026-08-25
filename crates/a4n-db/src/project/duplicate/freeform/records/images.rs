use a4n_core::models::ai::generated_image;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::project::duplicate::{
    ProjectDuplicateError, freeform::FreeformGeneratedImageCopyRecords,
    shared::ProjectMediaFileCopier,
};

pub(super) async fn copy_freeform_generated_images_record<C, Copier>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    now: NaiveDateTime,
    media_file_copier: &mut Copier,
) -> Result<FreeformGeneratedImageCopyRecords, ProjectDuplicateError>
where
    C: ConnectionTrait,
    Copier: ProjectMediaFileCopier,
{
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(source_project_id))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if images.is_empty() {
        return Ok(FreeformGeneratedImageCopyRecords::default());
    }
    let mut file_paths = Vec::with_capacity(images.len());
    let mut rows = Vec::with_capacity(images.len());
    for image in &images {
        let file_path = media_file_copier
            .copy_media_file(
                new_project_id,
                image.id,
                "image",
                Some(image.file_path.as_str()),
            )
            .map_err(ProjectDuplicateError::MediaCopy)?
            .ok_or_else(|| ProjectDuplicateError::InvalidState {
                current: format!("生成图片 {} 缺少 file_path", image.id),
                operation: "复制自由视频项目生成图片".to_string(),
            })?;
        file_paths.push(file_path.clone());
        rows.push(generated_image::ActiveModel {
            project_id: Set(new_project_id),
            image_type: Set(image.image_type.clone()),
            file_path: Set(file_path),
            prompt: Set(image.prompt.clone()),
            model: Set(image.model.clone()),
            width: Set(image.width),
            height: Set(image.height),
            file_size: Set(image.file_size),
            ai_score: Set(image.ai_score),
            is_favorite: Set(image.is_favorite),
            chapter_id: Set(None),
            panel_id: Set(image.panel_id.clone()),
            metadata_json: Set(image.metadata_json.clone()),
            created_at: Set(now),
            ..Default::default()
        });
    }
    let inserted = generated_image::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != images.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的生成图片数量与源记录不一致".to_string(),
            operation: "复制自由视频项目生成图片".to_string(),
        });
    }
    let mut copied = FreeformGeneratedImageCopyRecords::default();
    for ((image, file_path), row) in images.iter().zip(file_paths).zip(inserted) {
        copied.file_paths.insert(image.id, file_path);
        copied.ids.insert(image.id, row.id);
    }
    Ok(copied)
}
